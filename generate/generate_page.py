"""
    Generate the HTML page for displaying TYPES OF ANTS based on ants.txt
"""

TAB_AMOUNT = 2


def main():
    ants = open("ants.txt", "r")
    html = open("index.html", "w")
    template = open("generate/index_template.html", "r")

    TAB = " " * TAB_AMOUNT

    for template_line in template.readlines():
        if (
            template_line.strip()
            == '<div id="ant-filler" style="column-count: 4"></div>'
        ):
            html.write(f'{TAB*2}<div id="ant-filler" style="column-count: 4">\n')
            for ant_line in ants.readlines():
                type_of_ant = ant_line.strip()
                html.write(f"{TAB*3}<div>{type_of_ant}</div>\n")
            html.write(f"{TAB*2}</div>\n")
        else:
            html.write(template_line)

    html.close()
    ants.close()


if __name__ == "__main__":
    main()
