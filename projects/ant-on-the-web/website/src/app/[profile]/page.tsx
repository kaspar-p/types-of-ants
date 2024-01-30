type ProfilePageProps = {
  readonly params: {
    readonly profile: string;
  };
};

export default function Profile(props: ProfilePageProps) {
  const { profile } = props.params;
  return (
    <div>
      <h1>{profile}</h1>
      <div>There ain&apos;t a user here</div>
    </div>
  );
}
