import sys

from PIL import Image
import numpy as np

from matplotlib import pyplot as plt

with Image.open(sys.argv[1]) as im:
    im.convert("RGB")
    im.save(sys.argv[2])