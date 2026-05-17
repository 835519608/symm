; symm Windows 便携安装包：可选目录、解压式释放文件，不写入注册表。
; 目录布局：根目录 symm.exe（GUI）；CLI 在 cli\symm-cli.exe；data\ 为库目录。
; CI：ISCC /DAppVersion=<semver> packaging\windows\symm-setup.iss

#ifndef AppVersion
#define AppVersion "0.1.0"
#endif

#define BuildDir "..\..\target\release"
#define MyAppName "symm"
#define MyAppPublisher "symm"
#define MyOutputBase "symm-setup-windows-x64"

[Setup]
AppId={{7C9E2A41-5B8D-4F1E-9C3A-2D6E8F0B1A4C}
AppName={#MyAppName}
AppVersion={#AppVersion}
AppVerName={#MyAppName} {#AppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
Uninstallable=no
CreateUninstallRegKey=no
UpdateUninstallLogAppName=no
DisableDirPage=no
UsePreviousAppDir=no
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=dist
OutputBaseFilename={#MyOutputBase}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
SetupIconFile=..\..\assets\icon.ico

[Languages]
Name: "chinesesimplified"; MessagesFile: "languages\ChineseSimplified.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
chinesesimplified.WelcomeLabel2=将把 symm（图形界面）与 symm-cli（命令行）解压到您选择的文件夹。%n%n根目录为 symm.exe；CLI 在 cli 子目录。不写入注册表。
chinesesimplified.FinishedLabel=安装完成。双击 symm.exe 或从开始菜单启动；命令行请使用 cli\symm-cli.exe（或 Scoop 的 symm-cli）。
english.WelcomeLabel2=This will extract symm (GUI) and symm-cli into the folder you choose.%n%nNo registry entries are created.
english.FinishedLabel=Launch symm.exe from the Start menu or desktop. For CLI, run cli\symm-cli.exe.

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon} (symm GUI)"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#BuildDir}\symm.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BuildDir}\symm-cli.exe"; DestDir: "{app}\cli"; Flags: ignoreversion

[Dirs]
Name: "{app}\data"; Permissions: users-modify

[Icons]
Name: "{group}\symm"; Filename: "{app}\symm.exe"; Comment: "symm 图形界面"
Name: "{autodesktop}\symm"; Filename: "{app}\symm.exe"; Tasks: desktopicon; Comment: "symm"

[Run]
Filename: "{app}\symm.exe"; Description: "{cm:LaunchProgram,symm}"; Flags: nowait postinstall skipifsilent unchecked
