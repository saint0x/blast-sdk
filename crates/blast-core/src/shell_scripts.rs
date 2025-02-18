use std::path::PathBuf;

/// Activation scripts for different shell types
#[derive(Debug, Clone)]
pub struct ActivationScripts {
    pub bash: String,
    pub fish: String,
    pub powershell: String,
}

impl ActivationScripts {
    pub fn generate(env_path: &PathBuf, env_name: &str) -> Self {
        Self {
            bash: Self::bash_zsh(env_path, env_name),
            fish: Self::fish(env_path, env_name),
            powershell: Self::powershell(env_path, env_name),
        }
    }

    fn bash_zsh(env_path: &PathBuf, env_name: &str) -> String {
        format!(
            r#"#!/bin/bash
# Blast environment activation script for bash/zsh

deactivate () {{
    # reset old environment variables
    if [ -n "$_OLD_BLAST_PATH" ] ; then
        PATH="$_OLD_BLAST_PATH"
        export PATH
        unset _OLD_BLAST_PATH
    fi

    if [ -n "$_OLD_BLAST_PYTHONPATH" ] ; then
        PYTHONPATH="$_OLD_BLAST_PYTHONPATH"
        export PYTHONPATH
        unset _OLD_BLAST_PYTHONPATH
    fi

    if [ -n "$_OLD_BLAST_PS1" ] ; then
        PS1="$_OLD_BLAST_PS1"
        export PS1
        unset _OLD_BLAST_PS1
    fi

    if [ -n "$BASH" -o -n "$ZSH_VERSION" ] ; then
        hash -r 2>/dev/null
    fi

    if [ ! "$1" = "nondestructive" ] ; then
        # Self destruct!
        unset -f deactivate
    fi

    # Unset environment variables
    unset BLAST_ENV_NAME
    unset BLAST_ENV_PATH
    unset BLAST_SOCKET_PATH
}}

# Save the old path
_OLD_BLAST_PATH="$PATH"
PATH="{}/bin:$PATH"
export PATH

# Save the old PYTHONPATH
_OLD_BLAST_PYTHONPATH="$PYTHONPATH"
PYTHONPATH="{}/lib/python/site-packages:$PYTHONPATH"
export PYTHONPATH

# Save the old PS1
_OLD_BLAST_PS1="${{PS1-}}"
PS1="(blast:{}) $PS1"
export PS1

# Set environment variables
export BLAST_ENV_NAME="{}"
export BLAST_ENV_PATH="{}"
export BLAST_SOCKET_PATH="/tmp/blast_{}.sock"

# Make sure to unalias deactivate if it exists
if [ "$(type -t deactivate)" = "alias" ] ; then
    unalias deactivate
fi

if [ -n "$BASH" -o -n "$ZSH_VERSION" ] ; then
    hash -r 2>/dev/null
fi"#,
            env_path.display(),
            env_path.display(),
            env_name,
            env_name,
            env_path.display(),
            env_name
        )
    }

    fn fish(env_path: &PathBuf, env_name: &str) -> String {
        format!(
            r#"# Blast environment activation script for fish

function deactivate  -d "Exit blast virtual environment and return to normal shell environment"
    # reset old environment variables
    if test -n "$_OLD_BLAST_PATH"
        set -gx PATH $_OLD_BLAST_PATH
        set -e _OLD_BLAST_PATH
    end

    if test -n "$_OLD_BLAST_PYTHONPATH"
        set -gx PYTHONPATH $_OLD_BLAST_PYTHONPATH
        set -e _OLD_BLAST_PYTHONPATH
    end

    if test -n "$_OLD_FISH_PROMPT_OVERRIDE"
        functions -e fish_prompt
        set -e _OLD_FISH_PROMPT_OVERRIDE
        functions -c _old_fish_prompt fish_prompt
        functions -e _old_fish_prompt
    end

    set -e BLAST_ENV_NAME
    set -e BLAST_ENV_PATH
    set -e BLAST_SOCKET_PATH

    if test "$argv[1]" != "nondestructive"
        functions -e deactivate
    end
end

# Save the old path
set -gx _OLD_BLAST_PATH $PATH
set -gx PATH "{}/bin" $PATH

# Save the old PYTHONPATH
set -gx _OLD_BLAST_PYTHONPATH $PYTHONPATH
set -gx PYTHONPATH "{}/lib/python/site-packages" $PYTHONPATH

# Save the old prompt
functions -c fish_prompt _old_fish_prompt
set -gx _OLD_FISH_PROMPT_OVERRIDE "$BLAST_ENV_PATH"

function fish_prompt
    echo -n "(blast:{}) "
    _old_fish_prompt
end

# Set environment variables
set -gx BLAST_ENV_NAME "{}"
set -gx BLAST_ENV_PATH "{}"
set -gx BLAST_SOCKET_PATH "/tmp/blast_{}.sock""#,
            env_path.display(),
            env_path.display(),
            env_name,
            env_name,
            env_path.display(),
            env_name
        )
    }

    fn powershell(env_path: &PathBuf, env_name: &str) -> String {
        format!(
            r#"# Blast environment activation script for PowerShell

function global:deactivate ([switch]$NonDestructive) {{
    if (Test-Path variable:_OLD_BLAST_PATH) {{
        $env:PATH = $variable:_OLD_BLAST_PATH
        Remove-Variable "_OLD_BLAST_PATH" -Scope global
    }}

    if (Test-Path variable:_OLD_BLAST_PYTHONPATH) {{
        $env:PYTHONPATH = $variable:_OLD_BLAST_PYTHONPATH
        Remove-Variable "_OLD_BLAST_PYTHONPATH" -Scope global
    }}

    if (Test-Path variable:_OLD_BLAST_PROMPT) {{
        $function:prompt = $variable:_OLD_BLAST_PROMPT
        Remove-Variable "_OLD_BLAST_PROMPT" -Scope global
    }}

    if (Test-Path env:BLAST_ENV_NAME) {{
        Remove-Item env:BLAST_ENV_NAME
    }}
    if (Test-Path env:BLAST_ENV_PATH) {{
        Remove-Item env:BLAST_ENV_PATH
    }}
    if (Test-Path env:BLAST_SOCKET_PATH) {{
        Remove-Item env:BLAST_SOCKET_PATH
    }}

    if (!$NonDestructive) {{
        # Self destruct!
        Remove-Item function:deactivate
    }}
}}

# Save the old path
$global:_OLD_BLAST_PATH = $env:PATH
$env:PATH = "{}\bin;" + $env:PATH

# Save the old PYTHONPATH
$global:_OLD_BLAST_PYTHONPATH = $env:PYTHONPATH
$env:PYTHONPATH = "{}\lib\python\site-packages;" + $env:PYTHONPATH

# Save the old prompt
$global:_OLD_BLAST_PROMPT = $function:prompt
$function:prompt = {{
    Write-Host "(blast:{}) " -NoNewline
    & $global:_OLD_BLAST_PROMPT
}}

# Set environment variables
$env:BLAST_ENV_NAME = "{}"
$env:BLAST_ENV_PATH = "{}"
$env:BLAST_SOCKET_PATH = "/tmp/blast_{}.sock""#,
            env_path.display(),
            env_path.display(),
            env_name,
            env_name,
            env_path.display(),
            env_name
        )
    }
} 