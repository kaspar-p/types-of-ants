SRC_DIRS = $(wildcard find -type d)
SRC_FILES = $(shell find src/ -type f -name '*' )

out/: node_modules/ $(SRC_DIRS) $(SRC_FILES)
	npm run build

node_modules/: package.json package-lock.json
	npm ci

clean:
	rm -rf node_modules out .next

.PHONY: clean
