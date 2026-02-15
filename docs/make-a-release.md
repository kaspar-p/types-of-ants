# Making a release

Just run `./run-dev.sh ant-releasing-ants-into-the-wild`. It does everything.

## Sending the emails

When a new release is made, make note of the release number, e.g. `38`.

```sql
select user_name, user_email, ant_release.ant_content
from ant_release
  join ant on ant.ant_id = ant_release.ant_id
  join registered_user on registered_user.user_id = ant.ant_user_id
  join (
    select user_id, user_email
    from (
      select registered_user.user_id, user_email, row_number() over (partition by registered_user.user_id order by user_email) as rn
      from registered_user
        left join registered_user_email on registered_user.user_id = registered_user_email.user_id
    ) T where T.rn = 1
  ) e on e.user_id = registered_user.user_id
where
  release_number = 38 and
  user_name != 'nobody' and user_name != 'kaspar'
order by user_email
;
```

And send an email like:

Sender: <ants@typesofants.org>

Subject:

```txt
typesofants.org ant release #release_number: you're included!
```

Content:

```txt
hi @user_name,

the team at typesofants.org is excited to tell you that some of your suggestions were included in the latest release, #release_number!

the accepted suggestions were:

  [suggestion 1]
  [suggestion 2]
  ...

thank you for your contributions!

with love,
  the typesofants.org team
```
