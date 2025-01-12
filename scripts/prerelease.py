#!/usr/bin/python
# Copyright Sebastian Wiesner <sebastian@swsnr.de>

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

"""
Prerelease script for cargo release.
"""

import os
import sys
from pathlib import Path
from typing import NamedTuple, Self
import xml.etree.ElementTree as etree

class Version(NamedTuple):
    major: int
    minor: int
    patch: int

    @classmethod
    def parse(cls, s: str) -> Self:
        [major, minor, patch] = [int(p) for p in s.split('.')]
        return cls(major, minor, patch)

    def __str__(self) -> str:
        return f'{self.major}.{self.minor}.{self.patch}'


def is_patch_release(prev_version: Version, new_version: Version) -> bool:
    return prev_version.major == new_version.major and \
        prev_version.minor == new_version.minor and \
        prev_version.patch != new_version.patch


def assert_no_releasenotes(new_version: Version):
    metadata_file = Path(os.environ['CRATE_ROOT']) / 'resources' / 'de.swsnr.turnon.metainfo.xml.in'
    tree = etree.parse(metadata_file)
    if tree.find('./releases/release[@version="next"]') is not None:
        raise ValueError('Upcoming release notes found; must do a major or minor release, not a patch release!')
    if tree.find(f'./releases/release[@version="{new_version}"]') is not None:
        raise ValueError('Release notes for next version found; must do a major or minor release, not a patch release!')


def update_releasenotes(new_version: Version, *, tag_name: str, date: str, dry_run: bool):
    metadata_file = Path(os.environ['CRATE_ROOT']) / 'resources' / 'de.swsnr.turnon.metainfo.xml.in'
    parser = etree.XMLParser(target=etree.TreeBuilder(insert_comments=True))
    tree = etree.parse(metadata_file, parser)
    next_release = tree.find('./releases/release[@version="next"]')
    if next_release is None:
        raise ValueError("Doing a major or minor release but no release notes found!")
    next_release.attrib['version'] = str(new_version)
    next_release.attrib['date'] = date
    next_release[-1].tail = next_release.text
    url = etree.SubElement(next_release, 'url')
    url.tail = next_release.tail
    url.text = f'https://github.com/swsnr/turnon/releases/tag/{tag_name}'
    if dry_run:
        etree.dump(next_release)
    else:
        tree.write(metadata_file, xml_declaration=True, encoding='utf-8')


def main():
    prev_version = Version.parse(os.environ['PREV_VERSION'])
    new_version = Version.parse(os.environ['NEW_VERSION'])
    dry_run = os.environ['DRY_RUN'] == 'true'
    if is_patch_release(prev_version, new_version):
        assert_no_releasenotes(new_version)
    else:
        [tag_name, date] = sys.argv[1:]
        update_releasenotes(new_version, tag_name=tag_name, date=date, dry_run=dry_run)


if __name__ == '__main__':
    main()
