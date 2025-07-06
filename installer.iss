[Setup]
AppName=ComicReader
AppVersion=0.1.5-rc3
DefaultDirName={pf}\ComicReader
DefaultGroupName=ComicReader
OutputDir=dist
OutputBaseFilename=ComicReaderInstaller
Compression=lzma
SolidCompression=yes

[Files]
Source: "target\release\comic_reader.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "comic_reader\assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\ComicReader"; Filename: "{app}\comic_reader.exe"
Name: "{group}\Uninstall ComicReader"; Filename: "{uninstallexe}"
