Option Explicit
Dim shell, files, base, command, config
If WScript.Arguments.Count <> 1 Then WScript.Quit 2
Set files = CreateObject("Scripting.FileSystemObject")
Set shell = CreateObject("WScript.Shell")
base = files.GetParentFolderName(WScript.ScriptFullName)
config = WScript.Arguments(0)
command = "powershell.exe -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy RemoteSigned -File " & Chr(34) & base & "\Switch-Earbuds.ps1" & Chr(34) & " -ConfigPath " & Chr(34) & config & Chr(34)
shell.Run command, 0, False
