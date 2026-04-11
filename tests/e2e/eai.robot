*** Settings ***
Library    Collections
Library    OperatingSystem
Library    Process
Library    String
Suite Setup    Prepare Suite
Test Setup    Reset Sandbox

*** Variables ***
${CONFIG_BODY}    [default]\nbackend = "claude-cli"\nconfirm = true\n\n[search]\nenabled = false\nengine = "ddg"\n

*** Test Cases ***
Dry Run Should Print Generated Command
    ${env}=    Build Base Env
    Set To Dictionary    ${env}    EAI_MOCK_COMMAND=echo E2E_DRY\\n// dry mode command
    ${result}=    Run Process    ${EAI_BIN}    --backend    claude-cli    --dry    list files modified today    env=${env}    stderr=STDOUT
    Should Be Equal As Integers    ${result.rc}    0
    Should Contain    ${result.stdout}    echo E2E_DRY
    Should Contain    ${result.stdout}    dry mode command

Execute Should Work In Native Shell
    ${env}=    Build Base Env
    ${shell}=    Set Variable    sh
    ${mock_cmd}=    Set Variable    printf 'E2E_EXEC_OK\\n'
    IF    ${IS_WINDOWS}
        ${shell}=    Set Variable    powershell
        ${mock_cmd}=    Set Variable    Write-Output E2E_EXEC_OK
    END
    ${full_mock}=    Catenate    SEPARATOR=    ${mock_cmd}\n// execution command
    Set To Dictionary    ${env}    EAI_MOCK_COMMAND=${full_mock}
    ${result}=    Run Process    ${EAI_BIN}    --backend    claude-cli    --no-confirm    --shell    ${shell}    print execution token    env=${env}
    Should Be Equal As Integers    ${result.rc}    0
    Should Contain    ${result.stdout}    E2E_EXEC_OK

Explain Mode Should Return Explanation
    ${env}=    Build Base Env
    Set To Dictionary    ${env}    EAI_MOCK_EXPLAIN=Explains command from robot e2e mock.
    ${result}=    Run Process    ${EAI_BIN}    --backend    claude-cli    --explain    ls -la    env=${env}    stderr=STDOUT
    Should Be Equal As Integers    ${result.rc}    0
    Should Contain    ${result.stdout}    Explains command from robot e2e mock.

History Should Persist Executions
    ${env}=    Build Base Env
    ${shell}=    Set Variable    sh
    ${mock_cmd}=    Set Variable    printf 'E2E_HISTORY_OK\\n'
    IF    ${IS_WINDOWS}
        ${shell}=    Set Variable    powershell
        ${mock_cmd}=    Set Variable    Write-Output E2E_HISTORY_OK
    END
    ${full_mock}=    Catenate    SEPARATOR=    ${mock_cmd}\n// history command
    Set To Dictionary    ${env}    EAI_MOCK_COMMAND=${full_mock}
    ${run}=    Run Process    ${EAI_BIN}    --backend    claude-cli    --no-confirm    --shell    ${shell}    create history e2e marker    env=${env}
    Should Be Equal As Integers    ${run.rc}    0
    ${history}=    Run Process    ${EAI_BIN}    history    --search    history e2e marker    env=${env}
    Should Be Equal As Integers    ${history.rc}    0
    Should Contain    ${history.stdout}    create history e2e marker
    Should Contain    ${history.stdout}    E2E_HISTORY_OK

*** Keywords ***
Prepare Suite
    ${root}=    Normalize Path    ${CURDIR}${/}..${/}..
    ${is_windows}=    Evaluate    sys.platform.startswith("win")    modules=sys
    Set Suite Variable    ${ROOT}    ${root}
    Set Suite Variable    ${IS_WINDOWS}    ${is_windows}
    ${base}=    Normalize Path    ${ROOT}${/}target${/}robot-e2e
    Remove Directory    ${base}    recursive=True
    Create Directory    ${base}
    Set Suite Variable    ${BASE_DIR}    ${base}
    ${bin}=    Set Variable    ${ROOT}${/}target${/}debug${/}eai
    IF    ${IS_WINDOWS}
        ${bin}=    Set Variable    ${ROOT}${/}target${/}debug${/}eai.exe
    END
    Set Suite Variable    ${EAI_BIN}    ${bin}

Reset Sandbox
    ${name}=    Replace String    ${TEST NAME}    ${SPACE}    _
    ${sandbox}=    Normalize Path    ${BASE_DIR}${/}${name}
    Remove Directory    ${sandbox}    recursive=True
    Create Directory    ${sandbox}
    Set Test Variable    ${SANDBOX}    ${sandbox}
    Prepare Config Directories
    Write Test Config

Prepare Config Directories
    ${cfg_root}=    Normalize Path    ${SANDBOX}${/}config-root
    ${data_root}=    Normalize Path    ${SANDBOX}${/}data-root
    Create Directory    ${cfg_root}${/}eai
    Create Directory    ${data_root}${/}eai
    Set Test Variable    ${CONFIG_ROOT}    ${cfg_root}
    Set Test Variable    ${DATA_ROOT}    ${data_root}
    IF    ${IS_WINDOWS}
        ${appdata}=    Normalize Path    ${SANDBOX}${/}appdata
        ${local}=    Normalize Path    ${SANDBOX}${/}localappdata
        Create Directory    ${appdata}
        Create Directory    ${local}
        Set Test Variable    ${APPDATA_DIR}    ${appdata}
        Set Test Variable    ${LOCALAPPDATA_DIR}    ${local}
    ELSE
        ${home}=    Normalize Path    ${SANDBOX}${/}home
        Create Directory    ${home}
        Set Test Variable    ${HOME_DIR}    ${home}
    END

Write Test Config
    Create File    ${CONFIG_ROOT}${/}eai${/}config.toml    ${CONFIG_BODY}

Build Base Env
    ${env}=    Evaluate    dict(os.environ)    modules=os
    Set To Dictionary    ${env}    EAI_CONFIG_DIR=${CONFIG_ROOT}
    Set To Dictionary    ${env}    EAI_DATA_DIR=${DATA_ROOT}
    Set To Dictionary    ${env}    EAI_MOCK_CLAUDE=1
    IF    ${IS_WINDOWS}
        Set To Dictionary    ${env}    APPDATA=${APPDATA_DIR}
        Set To Dictionary    ${env}    LOCALAPPDATA=${LOCALAPPDATA_DIR}
        Set To Dictionary    ${env}    USERPROFILE=${SANDBOX}
    ELSE
        Set To Dictionary    ${env}    HOME=${HOME_DIR}
    END
    RETURN    ${env}
