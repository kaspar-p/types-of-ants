cp README.md README_OLD.md
cp index.html index_OLD.html

python3 generate/generate_page.py
python3 generate/generate_readme.py

function join_by {
  local d=${1-} f=${2-}
  if shift 2; then
    printf %s "$f" "${@/#/$d}"
  fi
}

diffed=()

diff_readme_result=$(diff README.md README_OLD.md)
if [[ "$diff_readme_result" != "" ]]; then
    diffed+=("README.md")
fi

diff_index_result=$(diff index.html index_OLD.html)
if [[ "$diff_index_result" != "" ]]; then
    diffed+=("index.html")
fi

if [[ ${#diffed[@]} -eq 0 ]]; then
    echo "Nothing changed"
else
    space_delimited="${diffed[@]}"
    comma_space_delimited="${space_delimited// /, }"
    echo "Updated $comma_space_delimited"
fi

rm README_OLD.md index_OLD.html