use std::time::Duration;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rustc_hash::FxHashMap;
use tracing::debug;

use blast_core::error::{BlastError, BlastResult};
use blast_core::package::{Package, PackageId, Version, VersionConstraint};
use pubgrub::range::Range;
use pubgrub::solver::Dependencies;
use crate::resolver::PubgrubVersion;

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

        let version = Version::parse(&data.info.version)
            .map_err(|e| BlastError::package(format!("Invalid version: {}", e)))?;

        let python_constraint = if let Some(requires_python) = data.info.requires_python {
            VersionConstraint::parse(&requires_python)
                .map_err(|e| BlastError::package(format!("Invalid Python version constraint: {}", e)))?
        } else {
            VersionConstraint::parse("*").unwrap()
        };

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

        let mut pkg = Package::new(
            PackageId::new(data.info.name, version),
            base_dependencies,
            python_constraint,
        );

        // Add additional metadata
        if let Some(desc) = data.info.description {
            pkg.set_description(desc);
        }
        if let Some(author) = data.info.author {
            pkg.set_author(author);
        }
        if let Some(homepage) = data.info.home_page {
            pkg.set_homepage(homepage);
        }

        // Add extras dependencies
        for (extra_name, deps) in extra_dependencies {
            pkg.set_extra_dependencies(extra_name, deps);
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
        version: &Version,
    ) -> BlastResult<Dependencies<String, PubgrubVersion>> {
        let deps = self.get_package_dependencies(package, version).await?;
        
        let mut ranges = FxHashMap::default();
        for (name, constraint) in deps {
            // Convert the version constraint to a PubGrub range
            let range = match constraint.to_string().as_str() {
                "*" => Range::any(),
                constraint => {
                    let version = Version::parse(constraint.trim_matches(|c| c == '*'))
                        .unwrap_or_else(|_| Version::parse("0.0.0").unwrap());
                    Range::exact(PubgrubVersion(version))
                }
            };
            
            ranges.insert(name, range);
        }
        Ok(Dependencies::Known(ranges))
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

        // Clean up version requirement
        let version_req = version_req.replace(" ", "")
            .replace("(", "")
            .replace(")", "");

        Ok(Self {
            package: package_name.to_string(),
            version_constraint: VersionConstraint::parse(&version_req)?,
            python_constraint,
            extras,
        })
    }
}

/// PyPI package resolver
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

    #[allow(dead_code)]
    pub async fn resolve_import(&self, import_name: &str) -> BlastResult<Option<Package>> {
        debug!("Resolving import: {}", import_name);
        
        // Check if we have a mapping for this import
        if let Some(package_name) = self.get_package_name(import_name) {
            // Get package metadata
            let metadata = self.get_package_info(package_name).await?;
            
            // Create package object
            let package = Package::new(
                PackageId::new(
                    metadata.name.clone(),
                    Version::parse(&metadata.version)?,
                ),
                self.parse_dependencies(&metadata.dependencies)?,
                self.parse_python_version(metadata.python_version.as_deref())?,
            );
            
            Ok(Some(package))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub async fn get_package_info(&self, name: &str) -> BlastResult<PackageMetadata> {
        // Check cache first
        if let Some(metadata) = self.package_cache.get(name) {
            return Ok(metadata.clone());
        }

        // Fetch from PyPI
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

        Ok(PackageMetadata {
            name: pypi_data.info.name,
            version: pypi_data.info.version,
            summary: pypi_data.info.summary,
            dependencies: pypi_data.info.requires_dist.unwrap_or_default(),
            python_version: pypi_data.info.requires_python,
            import_names: vec![name.to_string()], // Basic assumption
        })
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
        self.resolve_import(import_name).await.is_ok()
    }

    #[allow(dead_code)]
    pub async fn resolve_imports(&self, imports: &[String]) -> BlastResult<Vec<Package>> {
        let mut packages = Vec::new();
        
        for import_name in imports {
            if let Some(package) = self.resolve_import(import_name).await? {
                packages.push(package);
            }
        }
        
        Ok(packages)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub summary: Option<String>,
    pub dependencies: Vec<String>,
    pub python_version: Option<String>,
    pub import_names: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn test_get_package_metadata() {
        let mock_server = MockServer::start().await;
        let client = PyPIClient::new(10, 30, false).unwrap();

        Mock::given(method("GET"))
            .and(path("/pypi/requests/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "info": {
                    "name": "requests",
                    "version": "2.28.2",
                    "summary": "Python HTTP for Humans",
                    "requires_python": ">=3.7",
                },
                "releases": {
                    "2.28.2": [{
                        "requires_dist": [
                            "charset-normalizer>=2,<4",
                            "idna>=2.5,<4",
                            "urllib3>=1.21.1,<1.27",
                            "certifi>=2017.4.17"
                        ],
                        "requires_python": ">=3.7",
                        "yanked": false
                    }]
                }
            })))
            .mount(&mock_server)
            .await;

        let metadata = client.get_package_metadata("requests").await;
        assert!(metadata.is_ok());
        let metadata = metadata.unwrap();
        assert_eq!(metadata.name(), "requests");
        assert_eq!(metadata.version().to_string(), "2.28.2");
    }

    #[tokio::test]
    async fn test_get_package_versions() {
        let client = PyPIClient::new(10, 30, false).unwrap();
        let versions = client.get_package_versions("requests").await;
        assert!(versions.is_ok());
        let versions = versions.unwrap();
        assert!(!versions.is_empty());
    }

    #[test]
    fn test_dependency_parsing() {
        // Test basic version constraint
        let dep = Dependency::parse("requests>=2.0.0").unwrap();
        assert_eq!(dep.package, "requests");
        assert!(dep.version_constraint.matches(&Version::parse("2.0.0").unwrap()));
        assert!(dep.version_constraint.matches(&Version::parse("2.1.0").unwrap()));
        assert!(!dep.version_constraint.matches(&Version::parse("1.9.0").unwrap()));

        // Test with extras
        let dep = Dependency::parse("django[bcrypt]>=3.0.0").unwrap();
        assert_eq!(dep.package, "django");
        assert_eq!(dep.extras, vec!["bcrypt"]);
        assert!(dep.version_constraint.matches(&Version::parse("3.0.0").unwrap()));

        // Test with Python version constraint
        let dep = Dependency::parse("flask>=2.0.0; python_version >= '3.7'").unwrap();
        assert_eq!(dep.package, "flask");
        assert!(dep.python_constraint.is_some());
        let python_constraint = dep.python_constraint.unwrap();
        assert!(python_constraint.matches(&Version::parse("3.7.0").unwrap()));
        assert!(python_constraint.matches(&Version::parse("3.8.0").unwrap()));

        // Test complex version constraints
        let dep = Dependency::parse("numpy>=1.20.0,<2.0.0").unwrap();
        assert_eq!(dep.package, "numpy");
        assert!(dep.version_constraint.matches(&Version::parse("1.20.0").unwrap()));
        assert!(dep.version_constraint.matches(&Version::parse("1.25.0").unwrap()));
        assert!(!dep.version_constraint.matches(&Version::parse("2.0.0").unwrap()));
    }
} 