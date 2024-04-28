"""
    Generate the README.md document for displaying TYPES OF ANTS based on ants.txt
"""

import sys
from lib import TAB, apply_ant_rule


def main(ants_txt_path: str, readme_path: str) -> None:
    ants = open(ants_txt_path, "r")
    readme = open(readme_path, "w")

    readme.write("# types of ants\n\n")
    readme.write("> For real documentation, see the [docs](./docs)\n\n")
    readme.write("<div>\n")
    for ant_line in ants.readlines():
        type_of_ant = apply_ant_rule(ant_line.strip())
        readme.write(f"{TAB}<div>{type_of_ant}</div>\n")
    readme.write("</div>\n")

    ants.close()
    readme.close()


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise "Need arguments <ants.txt path> <readme path>"

    ants_txt_path = sys.argv[1]
    readme_path = sys.argv[2]
    main(ants_txt_path, readme_path)
