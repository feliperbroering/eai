*** Settings ***
Library    Collections
Library    OperatingSystem
Library    Process
Library    String

*** Test Cases ***
Windows Install Should Override Old Eai In Session
    ${is_windows}=    Evaluate    sys.platform.startswith("win")    modules=sys
    Run Keyword If    not ${is_windows}    Pass Execution    Windows-only test.

    ${root}=    Normalize Path    ${CURDIR}${/}..${/}..${/}target${/}robot-install-flow
    ${old_bin}=    Normalize Path    ${root}${/}oldbin
    ${new_bin}=    Normalize Path    ${root}${/}newbin
    Remove Directory    ${root}    recursive=True
    Create Directory    ${old_bin}
    Create Directory    ${new_bin}
    Create File    ${old_bin}${/}eai.cmd    @echo off\n\necho eai 0.0.0-old\n

    ${env}=    Evaluate    dict(os.environ)    modules=os
    ${path}=    Evaluate    r"${old_bin}" + ";" + os.environ.get("PATH", "")    modules=os
    Set To Dictionary    ${env}    PATH=${path}

    ${ps}=    Catenate    SEPARATOR=\n
    ...    $ErrorActionPreference = 'Stop'
    ...    $originalUserPath = [Environment]::GetEnvironmentVariable('Path','User')
    ...    try {
    ...      $env:EAI_INSTALL_DIR = '${new_bin}'
    ...      iwr https://raw.githubusercontent.com/feliperbroering/eai/main/install.ps1 -UseBasicParsing | iex
    ...      $resolved = (Get-Command eai).Source
    ...      $version = (& eai --version)
    ...      Write-Output ('RESOLVED=' + $resolved)
    ...      Write-Output ('VERSION=' + $version)
    ...    } finally {
    ...      [Environment]::SetEnvironmentVariable('Path', $originalUserPath, 'User')
    ...    }

    ${result}=    Run Process    powershell    -NoProfile    -ExecutionPolicy    Bypass    -Command    ${ps}    env=${env}
    Should Be Equal As Integers    ${result.rc}    0
    Should Contain    ${result.stdout}    RESOLVED=${new_bin}
    Should Contain    ${result.stdout}    VERSION=eai
    Should Not Contain    ${result.stdout}    0.0.0-old
