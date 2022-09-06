package main

import (
	"bufio"
	"fmt"
	"io/ioutil"
	"math/rand"
	"os"
	"strings"
	"time"
)

var EXIT_STATEMENT string = ".done"
var FILE_NAME string = "./ants.txt"

func insert(a []string, index int, value string) []string {
	if len(a) == index { // nil or empty slice or after last element
		return append(a, value)
	}
	a = append(a[:index+1], a[index:]...) // index < len(a)
	a[index] = value
	return a
}

func main() {
	rand.Seed(time.Now().Unix())

	// Make sure there are no args
	if len(os.Args[1:]) > 0 {
		fmt.Println("Invalid arguments! This takes no arguments!")
		os.Exit(1)
	}

	// Open the ants.txt file
	file, err := os.OpenFile(FILE_NAME, os.O_RDWR, os.ModePerm)
	if err != nil {
		fmt.Printf("Error: %s", err)
		panic(err)
	}
	defer file.Close()

	// Get all ants into a slice
	fScanner := bufio.NewScanner(file)
	var lines = make([]string, 0)
	for fScanner.Scan() {
		lines = append(lines, fScanner.Text())
	}

	if len(lines) == 0 {
		fmt.Println("Unable to read any lines from ants.txt!")
		os.Exit(1)
	}

	// Loop until exit statement made
	var antsToAdd = make([]string, 0)
	inScanner := bufio.NewScanner(os.Stdin)
	fmt.Printf("Time to add some ants! Type '%s' rather than an ant name to finish.\n", EXIT_STATEMENT)
	for {
		// Ask the user for ants to add
		fmt.Print("Add ant: ")
		inScanner.Scan()
		ant := inScanner.Text()

		if ant == EXIT_STATEMENT {
			break
		} else {
			antsToAdd = append(antsToAdd, ant)
		}
	}

	fmt.Printf("Adding ants: %s!\n", antsToAdd)

	// Insert the ants randomly into the slice
	for _, antToAdd := range antsToAdd {
		randomIndex := rand.Intn(len(lines)) % len(lines)
		lines = insert(lines, randomIndex, antToAdd)
	}

	err = ioutil.WriteFile(FILE_NAME, []byte(strings.Join(lines, "\n")+"\n"), 0644)
	if err != nil {
		fmt.Printf("Error writing to file: %s", err)
		os.Exit(1)
	}
	file.Sync()

	fmt.Printf("Added %d ants into file!\n", len(antsToAdd))
}
