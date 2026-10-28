import sys
from PIL import Image, ImageDraw
K = 2                      # 2倍で描いて縮小し、輪郭を滑らかにする
def s(v): return int(v * K)
W = s(1024)
im = Image.new("RGBA", (W, W), (0, 0, 0, 0))

# 背景: 上から下へ紺→青緑のグラデーション (角丸)
bg = Image.new("RGBA", (W, W))
top, bot = (28, 52, 92), (36, 122, 128)
px = bg.load()
for y in range(W):
    t = y / (W - 1)
    c = tuple(int(top[i] + (bot[i] - top[i]) * t) for i in range(3)) + (255,)
    for x in range(W): px[x, y] = c
mask = Image.new("L", (W, W), 0)
ImageDraw.Draw(mask).rounded_rectangle((s(64), s(64), s(960), s(960)), s(200), fill=255)
im.paste(bg, (0, 0), mask)
d = ImageDraw.Draw(im)

def rr(box, r, **kw): d.rounded_rectangle(tuple(s(v) for v in box), s(r), **kw)

# 床の影
sh = Image.new('RGBA', (W, W), (0, 0, 0, 0))
ImageDraw.Draw(sh).ellipse((s(190), s(790), s(834), s(870)), fill=(0, 0, 0, 80))
im = Image.alpha_composite(im, sh)
d = ImageDraw.Draw(im)

# 持ち手 (金属)
rr((352, 232, 672, 440), 80, outline=(206, 212, 220, 255), width=s(44))
rr((366, 246, 658, 440), 66, outline=(150, 158, 170, 255), width=s(8))

# 本体: 下段 (濃い赤) と 上段のフタ (明るい赤)
rr((164, 400, 860, 800), 56, fill=(176, 40, 32, 255))
rr((164, 400, 860, 560), 56, fill=(226, 66, 52, 255))
d.rectangle((s(164), s(500), s(860), s(560)), fill=(226, 66, 52, 255))
d.rectangle((s(164), s(556), s(860), s(574)), fill=(120, 24, 20, 255))   # フタの継ぎ目
rr((190, 420, 834, 452), 16, fill=(255, 140, 120, 140))                 # ハイライト

# 左右の金具
for x0 in (164, 806):
    rr((x0, 440, x0 + 54, 760), 14, fill=(190, 196, 206, 255))
    for y in (480, 720):
        d.ellipse((s(x0 + 15), s(y - 12), s(x0 + 39), s(y + 12)), fill=(110, 118, 130, 255))

# 留め金 (中央)
rr((462, 520, 562, 596), 18, fill=(214, 220, 228, 255), outline=(110, 118, 130, 255), width=s(8))
rr((482, 546, 542, 570), 12, fill=(70, 76, 88, 255))

wr = Image.new('RGBA', (s(400), s(400)), (0, 0, 0, 0))
w = ImageDraw.Draw(wr)
c = (252, 238, 214, 255)
w.rounded_rectangle((s(180), s(90), s(220), s(310)), s(10), fill=c)
w.ellipse((s(148), s(38), s(252), s(142)), fill=c)
w.rectangle((s(178), s(20), s(222), s(92)), fill=(0, 0, 0, 0))
w.ellipse((s(160), s(270), s(240), s(350)), fill=c)
w.ellipse((s(184), s(294), s(216), s(326)), fill=(0, 0, 0, 0))
wr = wr.rotate(45, resample=Image.BICUBIC).resize((s(300), s(300)), Image.LANCZOS)
im.alpha_composite(wr, (s(512 - 150), s(702 - 150)))
im = im.resize((1024, 1024), Image.LANCZOS)
im.save(sys.argv[1])
