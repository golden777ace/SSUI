"""SSUI — GPU-ускоренная библиотека графического интерфейса для Windows.

Ядро на Rust: Direct3D 11, Direct2D, DirectWrite.
Публичный API описан в `__init__.pyi`.
"""

from ._ssui import Ctx as Ctx
from ._ssui import Dlg as Dlg
from ._ssui import Fnt as Fnt
from ._ssui import Fx as Fx
from ._ssui import N as N
from ._ssui import Rct as Rct
from ._ssui import S as S
from ._ssui import W as W
from ._ssui import sgnl as sgnl

__all__ = [
    "W",
    "S",
    "N",
    "Ctx",
    "Fx",
    "Fnt",
    "Dlg",
    "Rct",
    "sgnl",
]

__version__ = "1.5.0"