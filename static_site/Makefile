all: cli/add

cli/add: cli/main.go cli/go.mod cli/go.sum
	cd cli && go build

clean:
	cd cli && rm add

PSEUDO: all clean add generate
