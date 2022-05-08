"""
    Generate the README.md document for displaying TYPES OF ANTS based on ants.txt
"""

TAB_AMOUNT = 2


def main():
    ants = open("ants.txt", "r")
    readme = open("README.md", "w")

    readme.write("# types of ants\n\n")
    readme.write("<div>\n")
    for ant_line in ants.readlines():
        type_of_ant = ant_line.strip()
        if "6krill" in type_of_ant:
            readme.write(
                f'{" "*TAB_AMOUNT}<div><a href="http://6krill.com">6krill ant</a></div>')
        else:
            readme.write(f"{' ' * TAB_AMOUNT}<div>{type_of_ant}</div>\n")
    readme.write("</div>\n")

    ants.close()
    readme.close()


if __name__ == "__main__":
    main()
