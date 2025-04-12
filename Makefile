
-include Makefile.common

BUILD_MODE = debug

PROJECTS = ant-on-the-web ant-who-tweets

all: debug

debug: $(PROJECTS)

release: RUST_FLAGS += --release
release: BUILD_MODE = release
release: $(PROJECTS)

$(PROJECTS)/release: %:
	$(MAKE) -C projects/$@ $(BUILD_MODE)

clean:
	echo $(PROJECTS)

.PHONY: all debug release clean $(PROJECTS)
