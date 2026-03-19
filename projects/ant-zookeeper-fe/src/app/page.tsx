import { Pipeline } from "@/components/Pipeline";
import { RefreshCounter } from "@/components/RefreshCounter";

export default async function Home() {
  const h1 = new Headers();
  h1.append("Content-Type", "application/json");

  const res = await fetch("http://localhost:3235/pipeline/pipelines");
  const body: { pipelineNames: string[] } = await res.json();

  const h2 = new Headers();
  h2.append("Content-Type", "application/json");

  const pipelines: string[] = body.pipelineNames;
  const responses: { pipeline: string; res: any }[] = [];

  for (const pipeline of pipelines) {
    const res = await fetch(
      `http://localhost:3235/pipeline/pipeline?name=${pipeline}`,
      {
        next: { revalidate: 2 },
        method: "GET",
        headers: h2,
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
