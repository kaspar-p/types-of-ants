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
  const responses: Record<string, any> = {};

  await Promise.all(
    pipelines.map(async (pipeline) => {
      const res = await fetch(
        `http://localhost:3235/pipeline/pipeline?name=${pipeline}`,
        {
          next: { revalidate: 2 },
          method: "GET",
          headers: h2,
        },
      ).then((x) => x.json());
      console.log(res.name, res.events);

      responses[pipeline] = res;
    }),
  );

  const responses2 = Object.entries(responses).toSorted((a, b) =>
    a[0] < b[0] ? -1 : 1,
  );

  return (
    <div className="flex flex-col space-y-4">
      <h1>zoo.typesofants.org</h1>
      <RefreshCounter />

      {responses2.map(([pipeline, res]) => (
        <Pipeline key={pipeline} res={res} />
      ))}
    </div>
  );
}
