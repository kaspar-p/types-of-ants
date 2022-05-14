"""
    Generate the HTML page for displaying TYPES OF ANTS based on ants.txt
"""
import os

TAB_AMOUNT = 2


def apply_ant_rule(ant: str) -> str:
    if "6krill" in ant:
        return '<a href="http://6krill.com">6krill ant</a>'
    if ant == "ant tm":
        print(f"Ant is '{ant}'")
        return "ant&trade;"

    return ant


def main():
    ants_f = open("ants.txt", "r")
    ants = [apply_ant_rule(ant.strip())
            for ant in ants_f.readlines()
            ]
    html = open("index.html", "w")
    template_f = open("generate/index_template.html", "r")
    template = template_f.readlines()

    TAB = " " * TAB_AMOUNT
    last_ants_change_git_hash = os.popen(
        'git log --follow -n 1 --pretty=format:"%h" --date=short ants.txt'
    ).readlines().pop()

    last_ants_change_git_date = os.popen(
        'git log --follow -n 1 --pretty=format:"%ad" --date=format:"%B %d, %Y" ants.txt'
    ).readlines().pop()

    commit_history_length = int(
        os.popen('git rev-list --count HEAD').readlines().pop())

    get_ants_changelist_command = f'git diff {last_ants_change_git_hash}^..HEAD --no-ext-diff --unified=0 --exit-code -a --no-prefix -- ants.txt | egrep "^\+" | cut -c2-'
    ant_changelist = os.popen(get_ants_changelist_command).readlines()[1:]
    ant_changelist = [apply_ant_rule(ant_line.strip())
                      for ant_line in ant_changelist]

    for template_line in template:
        # Inject contents of ants.txt
        if (
            template_line.strip()
            == '<div id="ant-filler" style="column-count: 4"></div>'
        ):
            html.write(
                f'{TAB*2}<div id="ant-filler" style="column-count: 4">\n')
            for ant in ants:
                html.write(f"{TAB*3}<div>{ant}</div>\n")
            html.write(f"{TAB*2}</div>\n")
        # Inject version number
        elif template_line.strip() == "v{amt}:":
            html.write(f"v{commit_history_length}: ")
        # Inject ant amount header
        elif template_line.strip() == '<h2>ants discovered to date: {amount}</h2>':
            html.write(
                f'<h2>ants discovered to date: {len(ants)}</h2>')
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
                    html.write(f"{ant}{'&nbsp;' * spaces_amt}")
            html.write(f"{TAB*5}</div>\n")
        else:
            html.write(template_line)

    html.close()
    template_f.close()
    ants_f.close()


if __name__ == "__main__":
    main()
