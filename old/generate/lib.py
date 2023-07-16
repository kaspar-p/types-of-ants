"""Generic library functions and constants that are used in the generation of the README.md
and in the generation of the index.html page.
"""

TAB_AMOUNT = 2
TAB = ' ' * TAB_AMOUNT


def apply_ant_rule(ant: str) -> str:
    """
    Takes an ant and applies a rule. 

    Args:
        ant (str): some ant, e.g. "6krill ant"

    Returns:
        str: transformed ant, e.g. "[6krill] ant" as a link
    """
    if "[6krill]" in ant:
        return '<a href="http://6krill.com">[6krill] ant</a>'
    if ant == "ant tm":
        return "ant&trade;"

    return ant
