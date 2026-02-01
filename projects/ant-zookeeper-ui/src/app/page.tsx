import Pipeline from "@/components/Pipeline";
import Image from "next/image";

export default async function Home() {
  const h = new Headers();

  h.append("Content-Type", "application/json");

  const resAntZooStorage = await fetch(
    "http://localhost:3235/pipeline/pipeline?project=ant-zoo-storage",
    {
      method: "GET",
      headers: h,
      // body: JSON.stringify({
      //   project: "ant-zoo-storage",
      // }),
    },
  ).then((x) => x.json());

  const resAntHostAgent = await fetch(
    "http://localhost:3235/pipeline/pipeline?project=ant-host-agent",
    {
      method: "GET",
      headers: h,
      // body: JSON.stringify({
      //   project: "ant-zoo-storage",
      // }),
    },
  ).then((x) => x.json());

  const res2 = await fetch("http://localhost:3235/deployment/iteration", {
    method: "POST",
    headers: h,
    body: JSON.stringify({
      // project: "ant-zoo-storage",
    }),
  }).then((x) => x.json());

  console.log({ resAntHostAgent, resAntZooStorage });

  return (
    <div className="flex flex-col space-y-4">
      <Pipeline res={resAntZooStorage} />
      <Pipeline res={resAntHostAgent} />
    </div>
  );
}
