"""
    Generate the HTML page for displaying TYPES OF ANTS based on ants.txt
"""
import os
from datetime import datetime
import time
from dataclasses import dataclass
import json
from typing import Union

from lib import TAB, apply_ant_rule

RELEASES_DIR = "./releases"
RELEASE_DATE_FILENAME_FORMAT = "%d%b%Y"


@dataclass
class AntRelease:
    ants: list[str]
    date: {"Year": int, "Month": str, "Day": int}


@dataclass
class ReleaseFile:
    date_filename: str
    path: str

    def pretty_format(self) -> str:
        """
        >>> f = ReleaseFile(date_filename="3Feb2024", path="./releases/3Feb2024.json")
        >>> f.pretty_format()
        'February 03, 2024'
        """
        dt = datetime.strptime(self.date_filename, RELEASE_DATE_FILENAME_FORMAT)
        return dt.strftime("%B %d, %Y")

    def get_ants(self) -> list[str]:
        f = open(self.path, "r")
        raw_data = json.load(f)
        return raw_data["Ants"]


def get_release_files() -> list[ReleaseFile]:
    files = os.listdir(RELEASES_DIR)
    return [
        ReleaseFile(file.split("/")[-1].split(".")[0], RELEASES_DIR + "/" + file) for file in files
    ]


def most_recent_release_filename(files: list[ReleaseFile]) -> Union[ReleaseFile, None]:
    """
    >>> files = [ReleaseFile("03Feb2024", "03Feb2024.json"), ReleaseFile("04Feb2024", "04Feb2024.json")]
    >>> most_recent_release_filename(files)
    ReleaseFile(date_filename='04Feb2024', path='04Feb2024.json')
    >>> files = [ReleaseFile("04Feb2024", "04Feb2024.json"), ReleaseFile("03Feb2024", "03Feb2024.json")]
    >>> most_recent_release_filename(files)
    ReleaseFile(date_filename='04Feb2024', path='04Feb2024.json')
    >>> files = [ReleaseFile("3Feb2024", "3Feb2024.json"), ReleaseFile("4Feb2024", "4Feb2024.json")]
    >>> most_recent_release_filename(files)
    ReleaseFile(date_filename='4Feb2024', path='4Feb2024.json')
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


def main():
    ants_f = open("ants.txt", "r")
    ants = [apply_ant_rule(ant.strip()) for ant in ants_f.readlines()]
    html = open("index.html", "w")
    template_f = open("generate/index_template.html", "r")
    template = template_f.readlines()

    release_files: list[ReleaseFile] = get_release_files()
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

    main()
