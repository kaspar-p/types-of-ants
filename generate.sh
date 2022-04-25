cp README.md README_OLD.md
cp index.html index_OLD.html

python3 generate/generate_page.py
python3 generate/generate_readme.py

diff_readme_result=$(diff README.md README_OLD.md)
diff_index_result=$(diff index.html index_OLD.html)
if [[ "$diff_readme_result" = "" ]]; then
    if [[ "$diff_index_result" = "" ]]; then
        echo "Nothing updated."
    else
        echo "index.html updated"
    fi
else
    if [[ "$diff_index_result" = "" ]]; then
        echo "README.md updated."
    else
        echo "Both index.html and README.md updated."
    fi
fi

rm README_OLD.md index_OLD.html