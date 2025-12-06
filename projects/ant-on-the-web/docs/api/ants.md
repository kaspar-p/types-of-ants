# ANTS `/api/v1/ants`

All data regarding ant suggestions and its associated metadata.

1. `POST /ListAllAnts`
   1. Get all ants and their associated metadata.
1. `POST /ListAntsWithStatus`
   1. Get all ants with a certain status.
1. `POST /ListLatestAnts`
   1. Get all of the ants that were a part of the latest release.
1. `POST /SuggestAnt`
   1. Suggest a new ant. Is suggested into the `unreleased` category of ants.
1. `POST /GetLatestRelease`
   1. Returns the latest release number.
