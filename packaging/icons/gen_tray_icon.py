import subprocess, sys
sizes = [22, 32, 48]
out = []
out.append("#pragma once")
out.append("// Generated from packaging/icons/hestia.svg. Raw ARGB32 (0xAARRGGBB,")
out.append("// big-endian byte order A,R,G,B) tray-icon pixmaps for the SNI IconPixmap")
out.append("// property, so the icon renders without relying on an installed icon theme.")
out.append("// Regenerate with packaging/icons/gen_tray_icon.py.")
out.append("#include <cstddef>")
out.append("namespace hestia::tray {")
out.append("struct TrayIconPixmap { int size; const unsigned char *argb; size_t len; };")
for s in sizes:
    raw = subprocess.run(
        ["magick","-background","none","packaging/icons/hestia.svg",
         "-resize", f"{s}x{s}","-depth","8","RGBA:-"],
        capture_output=True, check=True).stdout
    assert len(raw) == s*s*4, (len(raw), s)
    argb = bytearray(len(raw))
    for i in range(0, len(raw), 4):
        r,g,b,a = raw[i],raw[i+1],raw[i+2],raw[i+3]
        argb[i],argb[i+1],argb[i+2],argb[i+3] = a,r,g,b
    body = ",".join(str(x) for x in argb)
    out.append(f"static const unsigned char kTrayIcon{s}[] = {{{body}}};")
arr = ",".join(f"{{{s}, kTrayIcon{s}, sizeof(kTrayIcon{s})}}" for s in sizes)
out.append(f"static const TrayIconPixmap kTrayIcons[] = {{{arr}}};")
out.append("}  // namespace hestia::tray")
open("apps/tray/src/tray_icon_data.h","w").write("\n".join(out)+"\n")
print("wrote apps/tray/src/tray_icon_data.h", sum(s*s*4 for s in sizes), "bytes of pixel data")
