# Legacy Data

Previously, all data was kept as Github issues. This is a Typescript project to convert all the Github data (and data currently on the website), into usable SQL that the database can own.

## Getting data from Github

```
gh issue list -L 10000 -s all -R kaspar-p/types-of-ants --json assignees,author,body,closed,closedAt,comments,createdAt,id,labels,milestone,number,state,title > DATA_FILE
```
