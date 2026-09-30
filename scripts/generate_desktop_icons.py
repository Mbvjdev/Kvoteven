"""Generate packaging icons from Kvoteven's original 24px pet artwork.

No external images, account data, metadata or font dependencies are used.
Requires Pillow; this is an offline maintainer tool, not a runtime dependency.
"""
from pathlib import Path
from PIL import Image, ImageDraw


def generate(root=None):
    root = root or Path(__file__).resolve().parents[1] / "desktop/src-tauri/icons"
    root.mkdir(parents=True, exist_ok=True)
    image = Image.new("RGBA", (24, 24), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    ink = (48, 71, 59, 255)
    body = (156, 212, 166, 255)
    def box(x, y, w, h, color):
        draw.rectangle((x, y, x+w-1, y+h-1), fill=color)
    for rect in ((5,3,3,6),(16,3,3,6),(6,6,12,3),(4,8,16,12),(3,10,18,8),(6,19,12,3),(6,21,4,2),(14,21,4,2),(20,15,3,4)):
        box(*rect, ink)
    for rect in ((6,5,1,4),(17,5,1,4),(7,7,10,3),(5,9,14,10),(4,11,16,6),(7,19,10,2),(20,15,1,3)):
        box(*rect, body)
    box(7,9,5,1,(197,232,203,255))
    for rect in ((6,16,2,1),(16,16,2,1)):
        box(*rect,(230,156,143,255))
    for rect in ((8,12,2,3),(14,12,2,3),(10,16,1,1),(13,16,1,1),(11,17,2,1)):
        box(*rect,ink)
    for name, size in (("32x32.png",32),("128x128.png",128),("128x128@2x.png",256)):
        image.resize((size,size),Image.Resampling.NEAREST).save(root/name)
    master = image.resize((1024,1024),Image.Resampling.NEAREST)
    master.save(root/"icon.icns")
    master.save(root/"icon.ico", sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
    print("Generated original pixel-pet icons (no account data).")


if __name__ == "__main__":
    generate()
