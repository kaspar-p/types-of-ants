"""
    Generate the HTML page for displaying TYPES OF ANTS based on ants.txt
"""

import sys
import os
from datetime import datetime
import time
from dataclasses import dataclass
import json
from typing import Union

from lib import TAB, apply_ant_rule

RELEASE_DATE_FILENAME_FORMAT = "%Y-%m-%d"


@dataclass
class AntRelease:
    ants: list[str]
    date: {"Year": int, "Month": int, "Day": int}


@dataclass
class ReleaseFile:
    date_filename: str
    path: str

    def pretty_format(self) -> str:
        """
        >>> f = ReleaseFile(date_filename="2024-02-03", path="./releases/2024-02-03.json")
        >>> f.pretty_format()
        'February 03, 2024'
        """
        dt = datetime.strptime(self.date_filename, RELEASE_DATE_FILENAME_FORMAT)
        return dt.strftime("%B %d, %Y")

    def get_ants(self) -> list[str]:
        f = open(self.path, "r")
        raw_data = json.load(f)
        return raw_data["Ants"]


def get_release_files(releases_dir: str) -> list[ReleaseFile]:
    files = os.listdir(releases_dir)
    return [
        ReleaseFile(file.split("/")[-1].split(".")[0], releases_dir + "/" + file) for file in files
    ]


def most_recent_release_filename(files: list[ReleaseFile]) -> Union[ReleaseFile, None]:
    """
    >>> files = [ReleaseFile("2024-02-03", "2024-02-03.json"), ReleaseFile("2024-02-04", "2024-02-04.json")]
    >>> most_recent_release_filename(files)
    ReleaseFile(date_filename='2024-02-04', path='2024-02-04.json')
    >>> files = [ReleaseFile("2024-02-04", "2024-02-04.json"), ReleaseFile("2024-02-03", "2024-02-03.json")]
    >>> most_recent_release_filename(files)
    ReleaseFile(date_filename='2024-02-04', path='2024-02-04.json')
    """

    now = int(time.mktime(datetime.now().timetuple()))

    most_recent = None
    most_recent_diff = None

    for file in files:
        time_struct = time.strptime(file.date_filename, RELEASE_DATE_FILENAME_FORMAT)
        unix_time = int(time.mktime(time_struct))
        diff = now - unix_time

        if most_recent is None or diff < most_recent_diff:
            most_recent = file
            most_recent_diff = diff

    return most_recent


def get_version_number() -> int:
    return int(os.popen("git rev-list --count HEAD").readlines().pop())


def main(ants_txt_path: str, releases_dir: str, index_template_path: str, index_path: str):
    ants_f = open(ants_txt_path, "r")
    ants = [apply_ant_rule(ant.strip()) for ant in ants_f.readlines()]
    html = open(index_path, "w")
    template_f = open(index_template_path, "r")
    template = template_f.readlines()

    release_files: list[ReleaseFile] = get_release_files(releases_dir)
    latest_release: ReleaseFile = most_recent_release_filename(release_files)

    ant_changelist = [apply_ant_rule(ant.strip()) for ant in latest_release.get_ants()]

    for template_line in template:
        # Inject contents of ants.txt
        if template_line.strip() == '<div id="ant-filler"></div>':
            html.write(f'{TAB*2}<div id="ant-filler">\n')
            for ant in ants:
                html.write(f"{TAB*3}<div>{ant}</div>\n")
            html.write(f"{TAB*2}</div>\n")
        # Inject banner contents
        elif template_line.strip() == '<div id="scroll-container"></div>':
            html.write(f'{TAB*3}<div id="scroll-container">\n')
            for ant in ant_changelist:
                html.write(f'{TAB*4}<div class="banner-ant">{ant}</div>\n')
            html.write(f"{TAB*3}</div>\n")
        # Inject version number into the main title
        elif (
            template_line.strip()
            == '<h1>types of ants <span style="font-size: 12pt;">v{amt}</span></h1>'
        ):
            html.write(
                f'{TAB*2}<h1>types of ants <span style="font-size: 12pt;">v{get_version_number()}</span></h1>\n'
            )
        # Inject ant amount header
        elif template_line.strip() == "<h2>ants discovered to date: {amount}</h2>":
            html.write(f"<h2>ants discovered to date: {len(ants)}</h2>")
        # Inject banner title
        elif template_line.strip() == "<div>discovered {amt} new ants on {date}:</div>":
            html.write(
                f"{TAB*3}<div>discovered {len(ant_changelist)} new ants on {latest_release.pretty_format()}:</div>\n"
            )
        else:
            html.write(template_line)

    html.close()
    template_f.close()
    ants_f.close()


if __name__ == "__main__":
    import doctest

    res = doctest.testmod()
    if res.failed > 0:
        exit(1)

    if len(sys.argv) != 2:
        raise Exception("Need argument: <repository root>")
    
    repo_root = sys.argv[1]

    if not os.path.isdir(repo_root):
        raise Exception("Repository root was not a directory: " + repo_root)

    ants_txt_path = os.path.join(repo_root, "ants.txt")
    if not os.path.isfile(ants_txt_path):
        raise Exception("Could not find ants.txt at: " + ants_txt_path)

    releases_dir = os.path.join(repo_root, "static_site", "releases")
    if not os.path.isdir(releases_dir):
        raise Exception("Could not find releases/ directory at: " + releases_dir)

    index_template_path = os.path.join(repo_root, "static_site", "generate", "index_template.html")
    if not os.path.isfile(index_template_path):
        raise Exception("Could not find index_template.html at: " + index_template_path)

    index_path = os.path.join(repo_root, "index.html")
    if not os.path.isfile(index_path):
        raise Exception("Could not find index.html at: " + index_path)

    main(ants_txt_path, releases_dir, index_template_path, index_path)
