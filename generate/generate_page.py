"""
    Generate the HTML page for displaying TYPES OF ANTS based on ants.txt
"""
import os

TAB_AMOUNT = 2


def apply_ant_rule(ant: str) -> str:
    if "6krill" in ant:
        return '<a href="http://6krill.com">6krill ant</a>'

    return ant


def main():
    ants = open("ants.txt", "r")
    html = open("index.html", "w")
    template = open("generate/index_template.html", "r")

    TAB = " " * TAB_AMOUNT
    last_ants_change_git_hash = os.popen(
        'git log --follow -n 1 --pretty=format:"%h" --date=short ants.txt'
    ).readlines().pop()

    last_ants_change_git_date = os.popen(
        'git log --follow -n 1 --pretty=format:"%ad" --date=format:"%B %d, %Y" ants.txt'
    ).readlines().pop()

    get_ants_changelist_command = f'git diff {last_ants_change_git_hash}^..HEAD --no-ext-diff --unified=0 --exit-code -a --no-prefix -- ants.txt | egrep "^\+" | cut -c2-'
    ant_changelist = os.popen(get_ants_changelist_command).readlines()[1:]
    ant_changelist = [line.strip() for line in ant_changelist]

    for template_line in template.readlines():
        # Inject contents of ants.txt
        if (
            template_line.strip()
            == '<div id="ant-filler" style="column-count: 4"></div>'
        ):
            html.write(
                f'{TAB*2}<div id="ant-filler" style="column-count: 4">\n')
            for ant_line in ants.readlines():
                type_of_ant = ant_line.strip()
                ant = apply_ant_rule(type_of_ant)
                html.write(f"{TAB*3}<div>{ant}</div>\n")
            html.write(f"{TAB*2}</div>\n")
        # Inject banner title
        elif template_line.strip() == '<div>discovered {amt} new ants on {date}:</div>':
            html.write(
                f"<div>discovered {len(ant_changelist)} new ants on {last_ants_change_git_date}:</div>"
            )
        # Inject banner contents
        elif template_line.strip() == '<div id="scroll-text"></div>':
            html.write(f'{TAB*5}<div id="scroll-text">\n')
            for _ in range(50):
                for ant in ant_changelist:
                    spaces_amt = max([10, 100 // len(ant_changelist)])
                    ant = apply_ant_rule(ant)
                    html.write(f"{ant}{'&nbsp;' * spaces_amt}")
            html.write(f"{TAB*5}</div>\n")

        else:
            html.write(template_line)

    html.close()
    ants.close()


if __name__ == "__main__":
    main()
