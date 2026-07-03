"""Wickra Screener — the data-driven multi-symbol scan core.

Build a :class:`Screener` from a spec JSON, drive it with command JSONs, and
read back scan reports. The same command protocol crosses every language
binding, so this Python front-end drives the exact same core as the native CLI.
"""

from ._wickra_screener import Screener, __version__

__all__ = ["Screener", "__version__"]
