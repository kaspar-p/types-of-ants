import { Pipeline } from "@/components/Pipeline";
import { RefreshTimer } from "@/components/RefreshTimer";

export default async function Home() {
  const h = new Headers();

  h.append("Content-Type", "application/json");

  const projects = [
    "ant-looking-pretty",
    "ant-on-the-web",
    "ant-host-agent",
    "ant-gateway",
  ];
  const responses: { project: string; res: any }[] = [];

  for (const project of projects) {
    const res = await fetch(
      `http://localhost:3235/pipeline/pipeline?project=${project}`,
      {
        next: { revalidate: 2 },
        method: "GET",
        headers: h,
        // body: JSON.stringify({
        //   project: "ant-zoo-storage",
        // }),
      },
    ).then((x) => x.json());
    console.log(res.project, res.events);

    responses.push({ project, res });
  }

  // const res2 = await fetch("http://localhost:3235/deployment/iteration", {
  //   method: "POST",
  //   headers: h,
  //   body: JSON.stringify({
  //     // project: "ant-zoo-storage",
  //   }),
  // }).then((x) => x.json());

  return (
    <div className="flex flex-col space-y-4">
      <h1>zoo.typesofants.org</h1>
      <div>
        <RefreshTimer />
      </div>

      {responses.map((p) => (
        <Pipeline key={p.project} res={p.res} />
      ))}
    </div>
  );
}
