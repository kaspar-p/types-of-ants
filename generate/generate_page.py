"""
    Generate the HTML page for displaying TYPES OF ANTS based on ants.txt
"""

TAB_AMOUNT = 2


def main():
    ants = open("ants.txt", "r")
    html = open("index.html", "w")

    TAB = " " * TAB_AMOUNT

    html.writelines(
        [
            "<!DOCTYPE html>\n",
            '<html lang="en">\n',
            f"{TAB}<head>\n",
            f'{TAB*2}<meta charset="utf-8" />\n',
            f'{TAB*2}<link rel="icon" href="%PUBLIC_URL%/favicon.ico" />\n',
            f'{TAB*2}<meta name="viewport" content="width=device-width, initial-scale=1" />\n',
            f'{TAB*2}<meta name="theme-color" content="#000000" />\n',
            f'{TAB*2}<meta\n{TAB*3}name="description"\n{TAB*3}content="An informative site about ants, created by kaspar poland."\n{TAB*2}/>\n',
            f"{TAB*2}<title>types of ants</title>\n",
            f"{TAB}</head>\n",
            f"{TAB}<body>\n",
            f"{TAB*2}<h1>types of ants</h1>\n",
            f'{TAB*2}<div style="column-count: 4">\n',
        ]
    )

    for ant_line in ants.readlines():
        type_of_ant = ant_line.strip()
        html.write(f"{TAB*3}<div>{type_of_ant}</div>\n")
    html.writelines([f"{TAB*2}</div>\n", f"{TAB}</body>\n", "</html>\n"])

    html.close()
    ants.close()


if __name__ == "__main__":
    main()
