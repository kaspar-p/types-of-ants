import { Pipeline } from "@/components/Pipeline";
import { RefreshCounter } from "@/components/RefreshCounter";

export default async function Home() {
  const h = new Headers();

  h.append("Content-Type", "application/json");

  const pipelines = [
    "ant-data-farm",
    "ant-gateway",
    "ant-naming-domains",
    "website",
    "agent",
  ];
  const responses: { pipeline: string; res: any }[] = [];

  for (const pipeline of pipelines) {
    const res = await fetch(
      `http://localhost:3235/pipeline/pipeline?name=${pipeline}`,
      {
        next: { revalidate: 2 },
        method: "GET",
        headers: h,
      },
    ).then((x) => x.json());
    console.log(res.name, res.events);

    responses.push({ pipeline, res });
  }

  // const res2 = await fetch("http://localhost:3235/deployment/iteration", {
  //   method: "POST",
  //   headers: h,
  //   body: JSON.stringify({
  //     // project: "ant-zookeeper-db",
  //   }),
  // }).then((x) => x.json());

  return (
    <div className="flex flex-col space-y-4">
      <h1>zoo.typesofants.org</h1>
      <RefreshCounter />

      {responses.map((p) => (
        <Pipeline key={p.pipeline} res={p.res} />
      ))}
    </div>
  );
}
