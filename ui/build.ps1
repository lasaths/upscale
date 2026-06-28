# MSVC 14.51 install may lack CRT libs; 14.44 has them.
$msvc = "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.44.35207"
$sdk = "C:\Program Files (x86)\Windows Kits\10"
$sdkVer = "10.0.26100.0"
$env:LIB = "$msvc\lib\x64;$sdk\Lib\$sdkVer\um\x64;$sdk\Lib\$sdkVer\ucrt\x64"
$env:INCLUDE = "$msvc\include;$sdk\Include\$sdkVer\ucrt;$sdk\Include\$sdkVer\um;$sdk\Include\$sdkVer\shared"
$env:PATH = "$msvc\bin\Hostx64\x64;$env:PATH"
$env:UPSCALE_GPU = if ($env:UPSCALE_GPU) { $env:UPSCALE_GPU } else { "1" }
cargo @args
