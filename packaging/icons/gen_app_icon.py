import subprocess

for s in [16, 32, 128, 256]:
    subprocess.run(
        ["magick", "-background", "none", "packaging/icons/hestia.svg",
         "-resize", f"{s}x{s}", f"packaging/icons/hestia-{s}.png"],
        check=True)
    print(f"wrote packaging/icons/hestia-{s}.png")
