; ==============================================================================
; SensiDoc Windows Standard Installer Setup Script (Inno Setup 6)
; ==============================================================================

#ifndef MyAppVersion
#define MyAppVersion "1.4.2"
#endif

#ifndef OutputSuffix
#define OutputSuffix ""
#endif

#ifndef TargetArch
#define TargetArch "x86_64"
#endif

#ifndef ArchInstallMode
#define ArchInstallMode "x64compatible"
#endif

#define MyAppName "SensiDoc"
#define MyAppPublisher "SensiDoc Team"
#define MyAppURL "https://github.com/meteor-ioi/SensiDoc"
#define MyAppExeName "sensidoc.exe"

#ifndef ExeSourcePath
#define ExeSourcePath "..\target\release\" + MyAppExeName
#endif

#ifndef RuntimeBinDir
#define RuntimeBinDir "..\bin\windows-" + TargetArch
#endif

[Setup]
AppId={{D37E64A5-F2B8-43C1-90F6-6B2A6B3D4C0E}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} v{#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir=..\dist
OutputBaseFilename=sensidoc-v{#MyAppVersion}-windows-{#TargetArch}{#OutputSuffix}-setup
SetupIconFile=..\assets\sensidoc_win.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode={#ArchInstallMode}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
SetupLogging=yes

[Languages]
Name: "chinesesimp"; MessagesFile: "ChineseSimplified.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#ExeSourcePath}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\web\*"; DestDir: "{app}\web"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\assets\sensidoc_win.ico"; DestDir: "{app}"; DestName: "sensidoc.ico"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
; llama.cpp 离线模型运行时 (按架构匹配的 llama-server.exe 与 DLL)
Source: "{#RuntimeBinDir}\*"; DestDir: "{app}\bin"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist
; ONNX Runtime 动态链接库 (若 target 构建目录下生成了 onnxruntime.dll 则一并收录)
Source: "..\target\release\onnxruntime.dll"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "{#ExeSourcePath}\..\onnxruntime.dll"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
#ifdef IncludeOcrModels
Source: "..\models\ocr\*"; DestDir: "{app}\models\ocr"; Flags: ignoreversion recursesubdirs createallsubdirs
#endif

[Dirs]
Name: "{app}\bin"
Name: "{app}\models"
#ifdef IncludeOcrModels
Name: "{app}\models\ocr"
#endif

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\sensidoc.ico"
Name: "{group}\卸载 {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\sensidoc.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
// 安装过程日志自动持久化归档至应用日志目录，确保即使安装中断或失败也能调取排查
procedure DeinitializeSetup();
var
  LogFile: String;
  TargetDir: String;
  DestFile: String;
begin
  LogFile := ExpandConstant('{log}');
  if (LogFile <> '') and FileExists(LogFile) then
  begin
    TargetDir := ExpandConstant('{userappdata}\SensiDoc\logs');
    ForceDirectories(TargetDir);
    DestFile := TargetDir + '\installer.log';
    FileCopy(LogFile, DestFile, False);
  end;
end;

