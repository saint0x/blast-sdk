use std::path::PathBuf;

/// Shell activation script templates for different shells
pub struct ShellScripts;

impl ShellScripts {
    /// Generate activation script for bash/zsh shells
    pub fn bash_zsh(env_path: &PathBuf, env_name: &str) -> String {
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

    if [ -n "$_OLD_BLAST_PS1" ] ; then
        PS1="$_OLD_BLAST_PS1"
        export PS1
        unset _OLD_BLAST_PS1
    fi

    if [ -n "$BLAST_ENV" ] ; then
        unset BLAST_ENV
    fi

    if [ -n "$BLAST_ENV_NAME" ] ; then
        unset BLAST_ENV_NAME
    fi

    if [ ! "$1" = "nondestructive" ] ; then
        # Self destruct!
        unset -f deactivate
    fi
}}

# unset irrelevant variables
deactivate nondestructive

_OLD_BLAST_PATH="$PATH"
_OLD_BLAST_PS1="${{PS1:-}}"
BLAST_ENV="{}"
BLAST_ENV_NAME="{}"

export BLAST_ENV
export BLAST_ENV_NAME
export _OLD_BLAST_PATH
export _OLD_BLAST_PS1

PATH="$BLAST_ENV/bin:$PATH"
export PATH

if [ -z "$BLAST_DISABLE_PROMPT" ] ; then
    PS1="(blast:$BLAST_ENV_NAME) $PS1"
    export PS1
fi

# This should detect bash and zsh, which have a hash command that must
# be called to get it to forget past commands.  Without forgetting
# past commands the $PATH changes we made may not be respected
if [ -n "$BASH" -o -n "$ZSH_VERSION" ] ; then
    hash -r 2>/dev/null
fi
"#,
            env_path.display(),
            env_name
        )
    }

    /// Generate activation script for fish shell
    pub fn fish(env_path: &PathBuf, env_name: &str) -> String {
        format!(
            r#"# Blast environment activation script for fish shell

function deactivate  -d "Exit blast environment and return to normal shell environment"
    # reset old environment variables
    if test -n "$_OLD_BLAST_PATH"
        set -gx PATH $_OLD_BLAST_PATH
        set -e _OLD_BLAST_PATH
    end

    if test -n "$_OLD_BLAST_PS1"
        functions -c $_OLD_BLAST_PS1 fish_prompt
        functions -e _OLD_BLAST_PS1
    end

    if test -n "$BLAST_ENV"
        set -e BLAST_ENV
    end

    if test -n "$BLAST_ENV_NAME"
        set -e BLAST_ENV_NAME
    end

    if test "$argv[1]" != "nondestructive"
        # Self destruct!
        functions -e deactivate
    end
end

# Unset irrelevant variables
deactivate nondestructive

set -gx BLAST_ENV "{}"
set -gx BLAST_ENV_NAME "{}"

set -gx _OLD_BLAST_PATH $PATH
set -gx PATH "$BLAST_ENV/bin" $PATH

if not set -q BLAST_DISABLE_PROMPT
    functions -c fish_prompt _old_fish_prompt
    function fish_prompt
        if test -n "(blast)"
            printf "%s%s" "(blast:$BLAST_ENV_NAME) " (_old_fish_prompt)
        else
            _old_fish_prompt
        end
    end
end
"#,
            env_path.display(),
            env_name
        )
    }

    /// Generate activation script for PowerShell
    pub fn powershell(env_path: &PathBuf, env_name: &str) -> String {
        format!(
            r#"# Blast environment activation script for PowerShell

function global:deactivate ([switch]$NonDestructive) {{
    if (Test-Path variable:_OLD_BLAST_PATH) {{
        $env:PATH = $_OLD_BLAST_PATH
        Remove-Variable "_OLD_BLAST_PATH" -Scope global
    }}

    if (Test-Path variable:_OLD_BLAST_PROMPT) {{
        $env:PROMPT = $_OLD_BLAST_PROMPT
        Remove-Variable "_OLD_BLAST_PROMPT" -Scope global
    }}

    if (Test-Path env:BLAST_ENV) {{
        Remove-Item env:BLAST_ENV
    }}

    if (Test-Path env:BLAST_ENV_NAME) {{
        Remove-Item env:BLAST_ENV_NAME
    }}

    if (!$NonDestructive) {{
        # Self destruct!
        Remove-Item function:deactivate
    }}
}}

deactivate -NonDestructive

$_OLD_BLAST_PATH = $env:PATH
$_OLD_BLAST_PROMPT = $env:PROMPT

$env:BLAST_ENV = "{}"
$env:BLAST_ENV_NAME = "{}"
$env:PATH = "$env:BLAST_ENV\bin;$env:PATH"

if (!(Test-Path env:BLAST_DISABLE_PROMPT)) {{
    $env:PROMPT = "(blast:$env:BLAST_ENV_NAME) " + $env:PROMPT
}}
"#,
            env_path.display(),
            env_name
        )
    }

    /// Generate activation script for csh shell
    pub fn csh(env_path: &PathBuf, env_name: &str) -> String {
        format!(
            r#"# Blast environment activation script for csh shell

alias deactivate 'test $?_OLD_BLAST_PATH != 0 && setenv PATH "$_OLD_BLAST_PATH" && unset _OLD_BLAST_PATH; rehash; test $?_OLD_BLAST_PROMPT != 0 && set prompt="$_OLD_BLAST_PROMPT" && unset _OLD_BLAST_PROMPT; unsetenv BLAST_ENV; unsetenv BLAST_ENV_NAME; test "\!:*" != "nondestructive" && unalias deactivate'

# Unset irrelevant variables
deactivate nondestructive

setenv BLAST_ENV "{}"
setenv BLAST_ENV_NAME "{}"

set _OLD_BLAST_PATH="$PATH"
setenv PATH "$BLAST_ENV/bin:$PATH"

if ("" != "") then
    set _OLD_BLAST_PROMPT="$prompt"
endif

if (! "$?BLAST_DISABLE_PROMPT") then
    set prompt = "(blast:$BLAST_ENV_NAME) $prompt"
endif

rehash
"#,
            env_path.display(),
            env_name
        )
    }
}

/// Represents the activation scripts for a blast environment
pub struct ActivationScripts {
    pub bash: String,
    pub fish: String,
    pub csh: String,
    pub powershell: String,
}

impl ActivationScripts {
    /// Generate all activation scripts for an environment
    pub fn generate(env_path: &PathBuf, env_name: &str) -> Self {
        Self {
            bash: ShellScripts::bash_zsh(env_path, env_name),
            fish: ShellScripts::fish(env_path, env_name),
            csh: ShellScripts::csh(env_path, env_name),
            powershell: ShellScripts::powershell(env_path, env_name),
        }
    }
} 