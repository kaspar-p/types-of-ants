"""
    Generate the README.md document for displaying TYPES OF ANTS based on ants.txt
"""

from lib import TAB_AMOUNT, apply_ant_rule


def main():
    ants = open("ants.txt", "r")
    readme = open("README.md", "w")

    readme.write("# types of ants\n\n")
    readme.write("<div>\n")
    for ant_line in ants.readlines():
        type_of_ant = apply_ant_rule(ant_line.strip())
        readme.write(f"{' ' * TAB_AMOUNT}<div>{type_of_ant}</div>\n")
    readme.write("</div>\n")

    ants.close()
    readme.close()


if __name__ == "__main__":
    main()
