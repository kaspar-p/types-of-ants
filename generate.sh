cp README.md README_OLD.md

python3 generate/generate_page.py
python3 generate/generate_readme.py

diff_result=$(diff README.md README_OLD.md)
if [[ "$diff_result" = "" ]]; then
    echo "Nothing changed."
else
    echo "Updated README.md and index.html page."
fi
rm README_OLD.md