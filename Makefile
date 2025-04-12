
BUILD_MODE = debug

PROJECTS = ant-on-the-web

all: debug

debug: $(PROJECTS)

release: RUST_FLAGS += --release
release: BUILD_MODE = release
release: $(PROJECTS)

$(PROJECTS): %:
	cd projects/$@ && $(MAKE) $(BUILD_MODE)

clean:
	echo $(PROJECTS)

remake: clean debug
.NOTPARALLEL: remake

.PHONY: all debug release clean $(PROJECTS)
