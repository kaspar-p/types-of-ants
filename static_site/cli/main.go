package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"math/rand"
	"os"
	"strings"
	"time"
)

var EXIT_STATEMENT string = ".done"
var RELEASES_DIR string = "./releases"
var FILE_NAME string = "./ants.txt"

func insert(a []string, index int, value string) []string {
	if len(a) == index { // nil or empty slice or after last element
		return append(a, value)
	}
	a = append(a[:index+1], a[index:]...) // index < len(a)
	a[index] = value
	return a
}

type ReleaseDate struct {
	Year  int
	Month string
	Day   int
}

type Release struct {
	Date ReleaseDate
	Ants []string
}

func make_release(ants []string, year int, month string, day int) ([]byte, error) {
	release := Release{
		Ants: ants,
		Date: ReleaseDate{
			Year:  year,
			Month: month,
			Day:   day,
		},
	}

	return json.Marshal(release)
}

func main() {
	rand.Seed(time.Now().Unix())

	year, _month, day := time.Now().Date()
	month := (_month.String())[:3]

	// Make sure there are no args
	if len(os.Args[1:]) > 0 {
		fmt.Println("Invalid arguments! This takes no arguments!")
		os.Exit(1)
	}

	// Open the ants.txt file
	ants_file, err := os.OpenFile(FILE_NAME, os.O_RDWR, os.ModePerm)
	if err != nil {
		fmt.Printf("Error opening ants file: %s", err)
		panic(err)
	}
	defer ants_file.Close()

	// Open the release.txt file
	release_file_name := RELEASES_DIR + "/" + fmt.Sprint(day) + month + fmt.Sprint(year) + ".json"
	release_file, err := os.Create(release_file_name)
	if err != nil {
		fmt.Printf("Error opening release file: %s", err)
		panic(err)
	}
	defer release_file.Close()

	// Get all ants into a slice
	fScanner := bufio.NewScanner(ants_file)
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
		fmt.Printf("[%d] Add ant: ", len(antsToAdd))
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

	rand.Seed(time.Now().UnixNano())
	rand.Shuffle(len(antsToAdd), func(i, j int) {
		antsToAdd[i], antsToAdd[j] = antsToAdd[j], antsToAdd[i]
	})

	release_bytes, err := make_release(antsToAdd, year, month, day)
	if err != nil {
		fmt.Printf("Error creating JSON structure: %s", err)
		os.Exit(1)
	}

	err = ioutil.WriteFile(release_file_name, release_bytes, 0644)
	if err != nil {
		fmt.Printf("Error writing to release file: %s", err)
		os.Exit(1)
	}
	release_file.Sync()
	fmt.Printf("Created new release file with %d ants!\n", len(antsToAdd))

	err = ioutil.WriteFile(FILE_NAME, []byte(strings.Join(lines, "\n")+"\n"), 0644)
	if err != nil {
		fmt.Printf("Error writing to file: %s", err)
		os.Exit(1)
	}
	ants_file.Sync()
	fmt.Printf("Added %d ants into file!\n", len(antsToAdd))
}
