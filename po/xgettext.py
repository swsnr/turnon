#!/usr/bin/env python
# Copyright Sebastian Wiesner <sebastian@swsnr.de>
#
# Licensed under the EUPL
#
# See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

"""Extract messages from sources with xgettext."""

from pathlib import Path
from subprocess import run

APPID = "de.swsnr.turnon"

XGETTEXT_CMD = [
    "xgettext",
    f"--package-name={APPID}",
    "--foreign-user",
    "--copyright-holder=Sebastian Wiesner <sebastian@swsnr.de>",
    "--sort-by-file",
    "--from-code=UTF-8",
    "--add-comments",
    "--keyword=_",
    "--keyword=C_:1c,2",
]


def main() -> None:
    """Run xgettext over our sources."""
    root = Path(__file__).parents[1]
    blueprints = list((root / "resources").glob("**/*.blp"))
    try:
        _ = run(
            [
                *XGETTEXT_CMD,
                "--language=C",
                "--output=po/.blp.pot",
                *(str(p.relative_to(root)) for p in blueprints),
            ],
            check=True,
            cwd=root,
        )
        _ = run(
            [
                *XGETTEXT_CMD,
                f"--output=po/{APPID}.pot",
                "--generated=po/.blp.pot",
                *(f"--reference={p}" for p in blueprints),
                f"{APPID}.metainfo.xml",
                f"{APPID}.desktop",
                "po/.blp.pot",
                *(str(p.relative_to(root)) for p in (root / "src").glob("**/*.py")),
            ],
            check=True,
            cwd=root,
        )
    finally:
        (root / "po" / ".blp.pot").unlink(missing_ok=True)


if __name__ == "__main__":
    main()
