use std::collections::HashMap;
use std::time::Duration;

use reqwest::Client;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use blast_core::error::{BlastError, BlastResult};
use blast_core::package::{Package, PackageId};
use blast_core::metadata::PackageMetadata;
use blast_core::{Version, VersionConstraint};
use blast_image::compression::{
    CompressionType, CompressionLevel, CompressionStrategy,
    NoopStrategy, ZstdStrategy, GzipStrategy,
};
use pubgrub::range::Range;
use pubgrub::solver::Dependencies;

const PYPI_BASE_URL: &str = "https://pypi.org/pypi";

/// PyPI API client
#[derive(Clone)]
pub struct PyPIClient {
    client: Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PyPIResponse {
    info: PackageInfo,
    releases: HashMap<String, Vec<ReleaseInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageInfo {
    name: String,
    version: String,
    requires_python: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    author: Option<String>,
    author_email: Option<String>,
    home_page: Option<String>,
    license: Option<String>,
    requires_dist: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseInfo {
    yanked: Option<bool>,
    requires_dist: Option<Vec<String>>,
    requires_python: Option<String>,
    filename: String,
    python_version: String,
    url: String,
}

// Helper function to convert reqwest errors to BlastError
fn handle_reqwest_error(err: reqwest::Error) -> BlastError {
    BlastError::network(err.to_string())
}

impl PyPIClient {
    /// Create a new PyPI client
    pub fn new(connect_timeout: u64, request_timeout: u64, verify_ssl: bool) -> BlastResult<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(connect_timeout))
            .timeout(Duration::from_secs(request_timeout))
            .danger_accept_invalid_certs(!verify_ssl)
            .build()
            .map_err(handle_reqwest_error)?;

        Ok(Self { client })
    }

    /// Get package metadata from PyPI
    pub async fn get_package_metadata(&self, package: &str) -> BlastResult<Package> {
        let url = format!("{}/{}/json", PYPI_BASE_URL, package);
        debug!("Fetching package metadata from {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(handle_reqwest_error)?;

        if !response.status().is_success() {
            return Err(BlastError::package(format!(
                "Package not found: {} (status: {})",
                package,
                response.status()
            )));
        }

        let data: PyPIResponse = response
            .json()
            .await
            .map_err(|e| BlastError::package(format!("Invalid package metadata: {}", e)))?;

        let python_constraint = data.info.requires_python.as_deref()
            .map(|v| VersionConstraint::parse(v))
            .transpose()?
            .unwrap_or_else(VersionConstraint::any);

        // Track dependencies by extras
        let mut base_dependencies = HashMap::new();
        let mut extra_dependencies = HashMap::new();

        if let Some(release) = data.releases.get(&data.info.version) {
            if let Some(first_release) = release.first() {
                if let Some(requires_dist) = &first_release.requires_dist {
                    for req in requires_dist {
                        if let Ok(dep) = Dependency::parse(req) {
                            // Only add dependency if Python version constraint is met
                            if dep.python_constraint.as_ref().map_or(true, |pc| pc.matches(&Version::parse("3.7.0").unwrap())) {
                                if dep.extras.is_empty() {
                                    // Base dependency
                                    base_dependencies.insert(dep.package, dep.version_constraint);
                                } else {
                                    // Extra dependency
                                    for extra in &dep.extras {
                                        extra_dependencies
                                            .entry(extra.clone())
                                            .or_insert_with(HashMap::new)
                                            .insert(dep.package.clone(), dep.version_constraint.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut metadata = PackageMetadata::new(
            data.info.name.clone(),
            data.info.version.clone(),
            base_dependencies.clone(),
            python_constraint.clone(),
        );

        // Add additional metadata
        metadata.description = data.info.description;
        metadata.author = data.info.author;
        metadata.homepage = data.info.home_page;
        metadata.license = data.info.license;

        let mut pkg = Package::new(
            data.info.name.clone(),
            data.info.version.clone(),
            metadata,
            python_constraint,
        )?;

        // Add extras dependencies
        for (extra_name, deps) in extra_dependencies {
            pkg.metadata_mut().extras.insert(extra_name, deps);
        }

        Ok(pkg)
    }

    /// Get available versions for a package
    pub async fn get_package_versions(&self, package: &str) -> BlastResult<Vec<Version>> {
        let url = format!("{}/{}/json", PYPI_BASE_URL, package);
        debug!("Fetching package versions from {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(handle_reqwest_error)?;

        if !response.status().is_success() {
            return Err(BlastError::package(format!(
                "Package not found: {} (status: {})",
                package,
                response.status()
            )));
        }

        let data: PyPIResponse = response
            .json()
            .await
            .map_err(|e| BlastError::package(format!("Invalid package metadata: {}", e)))?;

        let mut versions = Vec::new();
        for (version_str, releases) in data.releases {
            // Skip yanked releases
            if releases.iter().any(|r| r.yanked.unwrap_or(false)) {
                continue;
            }

            if let Ok(version) = Version::parse(&version_str) {
                versions.push(version);
            }
        }

        versions.sort();
        Ok(versions)
    }

    /// Get package dependencies
    pub async fn get_package_dependencies(&self, package: &str, version: &Version) -> BlastResult<HashMap<String, VersionConstraint>> {
        let url = format!("{}/{}/{}/json", PYPI_BASE_URL, package, version);
        debug!("Fetching package dependencies from {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(handle_reqwest_error)?;

        if !response.status().is_success() {
            return Err(BlastError::package(format!(
                "Package not found: {}=={} (status: {})",
                package,
                version,
                response.status()
            )));
        }

        let data: PyPIResponse = response
            .json()
            .await
            .map_err(|e| BlastError::package(format!("Invalid package metadata: {}", e)))?;

        let mut dependencies = HashMap::new();
        if let Some(release) = data.releases.get(&version.to_string()) {
            if let Some(first_release) = release.first() {
                if let Some(requires_dist) = &first_release.requires_dist {
                    for req in requires_dist {
                        if let Some((name, constraint)) = parse_requirement(req) {
                            dependencies.insert(name, constraint);
                        }
                    }
                }
            }
        }

        Ok(dependencies)
    }

    pub async fn get_dependencies(
        &self,
        package: &str,
        version: &str,
    ) -> BlastResult<Dependencies<String, PyPIVersion>> {
        let url = format!("{}/{}/{}/json", PYPI_BASE_URL, package, version);
        debug!("Fetching package dependencies from {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(handle_reqwest_error)?;

        if !response.status().is_success() {
            return Err(BlastError::package(format!(
                "Package not found: {}=={} (status: {})",
                package,
                version,
                response.status()
            )));
        }

        let data: PyPIResponse = response
            .json()
            .await
            .map_err(|e| BlastError::package(format!("Invalid package metadata: {}", e)))?;

        let mut dependencies = HashMap::new();
        if let Some(release) = data.releases.get(version) {
            if let Some(first_release) = release.first() {
                if let Some(requires_dist) = &first_release.requires_dist {
                    for req in requires_dist {
                        if let Some((name, constraint)) = parse_requirement(req) {
                            dependencies.insert(name, constraint);
                        }
                    }
                }
            }
        }

        let mut ranges = FxHashMap::default();
        for (name, constraint) in dependencies {
            let range = match constraint.to_string().as_str() {
                "*" => Range::any(),
                constraint => {
                    let version = Version::parse(constraint.trim_matches(|c| c == '*'))
                        .unwrap_or_else(|_| Version::parse("0.0.0").unwrap());
                    Range::exact(PyPIVersion(version))
                }
            };
            
            ranges.insert(name, range);
        }
        Ok(Dependencies::Known(ranges))
    }

    #[allow(dead_code)]
    async fn get_package(&self, package_id: &PackageId) -> BlastResult<Package> {
        let metadata = self.get_package_metadata(package_id.name()).await?;
        let _base_dependencies = self.get_dependencies(package_id.name(), &package_id.version().to_string()).await?;

        let pkg = Package::new(
            package_id.name().to_string(),
            package_id.version().to_string(),
            metadata.metadata().clone(),
            metadata.metadata().python_version.clone(),
        )?;

        Ok(pkg)
    }

    pub async fn resolve_import(&self, import_name: &str) -> BlastResult<Option<Package>> {
        let package_name = match self.get_package_metadata(import_name).await {
            Ok(metadata) => metadata.name().to_string(),
            Err(_) => return Ok(None),
        };

        let client = PyPIClient::new(10, 30, false)?;
        let metadata = client.get_package_metadata(&package_name).await?;
        let _base_dependencies = client.get_dependencies(&package_name, metadata.version().to_string().as_str()).await?;

        Package::new(
            metadata.name().to_string(),
            metadata.version().to_string(),
            metadata.metadata().clone(),
            metadata.metadata().python_version.clone(),
        ).map(Some)
    }

    pub async fn is_available(&self, import_name: &str) -> bool {
        if let Some(package_name) = self.get_package_name(import_name) {
            if let Ok(client) = PyPIClient::new(10, 30, false) {
                client.get_package_metadata(package_name.as_str()).await.is_ok()
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn get_package_name(&self, import_name: &str) -> Option<String> {
        // Basic implementation - in real world this would need a more sophisticated mapping
        Some(import_name.to_string())
    }

    #[allow(dead_code)]
    async fn get_package_info(&self, name: &str) -> BlastResult<PackageMetadata> {
        let url = format!("{}/{}/json", PYPI_BASE_URL, name);
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(handle_reqwest_error)?;

        if !response.status().is_success() {
            return Err(BlastError::package(format!(
                "Package '{}' not found on PyPI",
                name
            )));
        }

        let pypi_data: PyPIResponse = response.json()
            .await
            .map_err(handle_reqwest_error)?;

        // Parse dependencies
        let mut dependencies = HashMap::new();
        if let Some(deps) = pypi_data.info.requires_dist {
            for dep_str in deps {
                if let Some((name, constraint)) = parse_requirement(&dep_str) {
                    dependencies.insert(name, constraint);
                }
            }
        }

        let python_constraint = pypi_data.info.requires_python
            .as_deref()
            .map(|v| VersionConstraint::parse(v))
            .transpose()?
            .unwrap_or_else(VersionConstraint::any);

        let mut metadata = PackageMetadata::new(
            pypi_data.info.name.clone(),
            pypi_data.info.version.clone(),
            dependencies,
            python_constraint,
        );

        metadata.description = pypi_data.info.description;
        metadata.author = pypi_data.info.author;
        metadata.homepage = pypi_data.info.home_page;
        metadata.license = pypi_data.info.license;

        Ok(metadata)
    }

    #[allow(dead_code)]
    async fn update_package_metadata(&self, package: &mut Package, info: &Value) -> BlastResult<()> {
        let mut metadata = package.metadata().clone();

        if let Some(description) = info["description"].as_str() {
            metadata.description = Some(description.to_string());
        }

        if let Some(author) = info["author"].as_str() {
            metadata.author = Some(author.to_string());
        }

        if let Some(homepage) = info["home_page"].as_str() {
            metadata.homepage = Some(homepage.to_string());
        }

        // Parse dependencies
        let mut dependencies = HashMap::new();
        if let Some(requires_dist) = info["requires_dist"].as_array() {
            for req in requires_dist {
                if let Some(req_str) = req.as_str() {
                    if let Some((name, constraint)) = parse_requirement(req_str) {
                        dependencies.insert(name, constraint);
                    }
                }
            }
        }

        // Update package with new metadata
        *package.metadata_mut() = metadata;

        Ok(())
    }
}

fn parse_requirement(req: &str) -> Option<(String, VersionConstraint)> {
    match Dependency::parse(req) {
        Ok(dep) => Some((dep.package, dep.version_constraint)),
        Err(_) => None
    }
}

/// Package dependency
#[derive(Debug, Clone)]
pub struct Dependency {
    pub package: String,
    pub version_constraint: VersionConstraint,
    pub python_constraint: Option<VersionConstraint>,
    pub extras: Vec<String>,
}

impl Dependency {
    /// Parse a dependency string according to PEP 508
    pub fn parse(dep_str: &str) -> BlastResult<Self> {
        // Split package name from version/extras
        let parts: Vec<&str> = dep_str.split(';').collect();
        let main_req = parts[0].trim();

        // Parse extras and version constraints
        let (package_name, extras, version_req) = if main_req.contains('[') {
            // Has extras
            let bracket_idx = main_req.find('[').unwrap();
            let close_idx = main_req.find(']')
                .ok_or_else(|| BlastError::package("Invalid extras format"))?;
            
            let name = main_req[..bracket_idx].trim();
            let extras_str = main_req[bracket_idx + 1..close_idx].trim();
            let extras: Vec<String> = extras_str.split(',')
                .map(|s| s.trim().to_string())
                .collect();

            let version_part = main_req[close_idx + 1..].trim();
            let version_req = if version_part.is_empty() {
                "*"
            } else {
                version_part.trim_start_matches(|c| c == '(' || c == '=' || c == ' ')
                    .trim_end_matches(|c| c == ')' || c == ' ')
            };

            (name, extras, version_req)
        } else if main_req.contains('=') || main_req.contains('>') || main_req.contains('<') || main_req.contains('~') {
            // Has version constraint
            let mut parts = main_req.splitn(2, |c| c == '=' || c == '>' || c == '<' || c == '~');
            let name = parts.next().unwrap().trim();
            let version_req = parts.next().unwrap_or("*").trim();
            (name, Vec::new(), version_req)
        } else {
            // Just package name
            (main_req, Vec::new(), "*")
        };

        // Parse Python version constraint if present
        let python_constraint = if parts.len() > 1 {
            let python_req = parts[1].trim();
            if python_req.starts_with("python_version") {
                let version_str = python_req.split_once(|c| c == '>' || c == '<' || c == '=')
                    .map(|(_, v)| v.trim())
                    .unwrap_or("*");
                Some(VersionConstraint::parse(version_str)?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            package: package_name.to_string(),
            version_constraint: VersionConstraint::parse(version_req)?,
            python_constraint,
            extras,
        })
    }
}

/// PyPI package resolver
#[allow(dead_code)]
pub struct PyPIResolver {
    client: Client,
    package_cache: HashMap<String, PackageMetadata>,
    import_map: HashMap<String, String>,
}

impl Default for PyPIResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl PyPIResolver {
    pub fn new() -> Self {
        let mut resolver = Self {
            client: Client::new(),
            package_cache: HashMap::new(),
            import_map: HashMap::new(),
        };
        
        // Add common import mappings
        resolver.add_default_mappings();
        resolver
    }

    fn add_default_mappings(&mut self) {
        let default_mappings = [
            ("numpy", "numpy"),
            ("pandas", "pandas"),
            ("requests", "requests"),
            ("tensorflow", "tensorflow"),
            ("torch", "torch"),
            ("sklearn", "scikit-learn"),
            ("matplotlib", "matplotlib"),
            ("pytest", "pytest"),
        ];

        for (import_name, package_name) in default_mappings {
            self.add_import_mapping(import_name.to_string(), package_name.to_string());
        }
    }

    pub fn add_import_mapping(&mut self, import_name: String, package_name: String) {
        debug!("Adding import mapping: {} -> {}", import_name, package_name);
        self.import_map.insert(import_name, package_name);
    }

    #[allow(dead_code)]
    pub fn get_package_name(&self, import_name: &str) -> Option<&String> {
        self.import_map.get(import_name)
    }

    #[allow(dead_code)]
    pub async fn is_available(&self, import_name: &str) -> bool {
        if let Some(package_name) = self.get_package_name(import_name) {
            if let Ok(client) = PyPIClient::new(10, 30, false) {
                client.get_package_metadata(package_name.as_str()).await.is_ok()
            } else {
                false
            }
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub async fn resolve_imports(&self, imports: &[String]) -> BlastResult<Vec<Package>> {
        let mut packages = Vec::new();
        let client = PyPIClient::new(10, 30, false)?;
        
        for import_name in imports {
            if let Some(package_name) = self.get_package_name(import_name) {
                match client.get_package_metadata(package_name).await {
                    Ok(package) => packages.push(package),
                    Err(_) => continue,
                }
            }
        }
        
        Ok(packages)
    }

    #[allow(dead_code)]
    fn parse_dependencies(&self, deps: &[String]) -> BlastResult<HashMap<String, VersionConstraint>> {
        let mut result = HashMap::new();
        
        for dep_str in deps {
            if let Some((name, constraint)) = parse_requirement(dep_str) {
                result.insert(name, constraint);
            }
        }
        
        Ok(result)
    }

    #[allow(dead_code)]
    fn parse_python_version(&self, version: Option<&str>) -> BlastResult<VersionConstraint> {
        match version {
            Some(v) => VersionConstraint::parse(v),
            None => Ok(VersionConstraint::any()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct PyPIVersion(pub(crate) Version);

impl std::fmt::Display for PyPIVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Version> for PyPIVersion {
    fn from(v: Version) -> Self {
        PyPIVersion(v)
    }
}

impl pubgrub::version::Version for PyPIVersion {
    fn lowest() -> Self {
        PyPIVersion(Version::parse("0.0.0").unwrap())
    }

    fn bump(&self) -> Self {
        // Since we can't access Version's internals directly,
        // we'll parse the version string and increment
        let version_str = self.0.to_string();
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() >= 3 {
            let major: u64 = parts[0].parse().unwrap_or(0);
            let minor: u64 = parts[1].parse().unwrap_or(0);
            let patch: u64 = parts[2].parse().unwrap_or(0);
            let new_version = format!("{}.{}.{}", major, minor, patch + 1);
            PyPIVersion(Version::parse(&new_version).unwrap())
        } else {
            PyPIVersion(Version::parse("0.0.1").unwrap())
        }
    }
}

#[allow(dead_code)]
pub fn create_strategy(
    compression_type: CompressionType,
    level: CompressionLevel,
) -> Box<dyn CompressionStrategy> {
    match compression_type {
        CompressionType::None => Box::new(NoopStrategy),
        CompressionType::Zstd => Box::new(ZstdStrategy::new(level)),
        CompressionType::Gzip => Box::new(GzipStrategy::new(level)),
    }
} 