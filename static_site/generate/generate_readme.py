"""
    Generate the README.md document for displaying TYPES OF ANTS based on ants.txt
"""

import sys
import os
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
    if len(sys.argv) != 2:
        raise "Need arguments <repository root>"

    repo_root = sys.argv[1]
    if not os.path.isdir(repo_root):
        raise "Repository root not a directory: " + repo_root
    
    ants_txt_path = os.path.join(repo_root, "ants.txt")
    if not os.path.isfile(ants_txt_path):
        raise "Could not find ants.txt at: " + ants_txt_path

    readme_path = os.path.join(repo_root, "README.md")
    if not os.path.isfile(readme_path):
        raise "Could not find README.md at: " + readme_path

    main(ants_txt_path, readme_path)
