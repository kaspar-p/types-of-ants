type ProfilePageProps = {
  readonly params: Promise<{
    readonly profile: string;
  }>;
};

export default async function Profile(props: ProfilePageProps) {
  const { profile } = await props.params;
  return (
    <div>
      <h3>{profile}</h3>
      <div>feature coming soon...</div>
    </div>
  );
}
