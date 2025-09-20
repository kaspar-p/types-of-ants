SRC_DIRS = $(wildcard find -type d)
SRC_FILES = $(shell find src/ -type f -name '*' )

out:
	@npm ci --silent >> /dev/stderr
	@npm run build

clean:
	rm -rf node_modules out .next

.PHONY: clean out
