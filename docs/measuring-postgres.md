# Measuring Postgres Queries

1. Launch a psql session with the database.
2. Load the "auto_explain" plugin to get queries over a certain threshold to
   explain themselves:

```sql
SET client_min_messages = log;

LOAD 'auto_explain';
SET auto_explain.log_min_duration=0;
SET auto_explain.log_analyze=true;
```

The copy-paste the query:

```sql
select
  ant.ant_id,
  ant.suggested_content,
  ant_release.ant_content,
  ant_release.ant_content_hash,
  ant_release.release_number,
  ant.created_at,
  registered_user.user_name,
  ant.ant_user_id,
  ant_declined.ant_declined_at,
  ant_tweeted.tweeted_at,
  release.release_label,
  release.release_number,
  release.created_at as release_created_at,
  release.creator_user_id
from
  ant left join ant_release on ant.ant_id = ant_release.ant_id
      left join ant_declined on ant.ant_id = ant_declined.ant_id
      left join ant_tweeted on ant.ant_id = ant_tweeted.ant_id
      left join registered_user on ant.ant_user_id = registered_user.user_id
      left join release on ant_release.release_number = release.release_number order by ant_release.ant_content_hash nulls first
```

Then feed everything into Claude and ask for help.
