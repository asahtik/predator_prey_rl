import sys

from PIL import Image
import numpy as np

from matplotlib import pyplot as plt

with Image.open(sys.argv[1]) as im:
    im.convert("RGB")
    im = np.asarray(im)
    out = np.zeros(im.shape[:2], dtype=np.uint8)
    for i in range(im.shape[0]):
        for j in range(im.shape[1]):
            if im[i, j, 0] == 0 and im[i, j, 1] == 80 and im[i, j, 2] == 0:
                out[i, j] = 1
            elif im[i, j, 0] == 255 and im[i, j, 1] == 50 and im[i, j, 2] == 0:
                out[i, j] = 2
            elif im[i, j, 0] == 0 and im[i, j, 1] == 90 and im[i, j, 2] == 255:
                out[i, j] = 3
    plt.imshow(out, cmap="gray")
    plt.show()
    out_im = Image.fromarray(out)
    out_im.save(sys.argv[2], format="bmp")