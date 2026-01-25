import Image from "next/image";

export default async function Home() {
  const h = new Headers();

  h.append("Content-Type", "application/json");

  const res = await fetch(
    "http://localhost:3235/pipeline/pipeline?project=ant-zoo-storage",
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

  console.log(res);

  return (
    <div>
      <h3>{res.project}</h3>
      <div className="flex flex-col space-y-4">
        {res.stages.map((stage, i: number) => (
          <div key={i}>
            <div className="flex flex-col space-y-2 border p-2">
              <div className="text-xl flex flex-row">
                {stage.stageName}
                <div className="ml-2 text-sm self-center">
                  (<i>type: {stage.stageType.type}</i>)
                </div>
              </div>

              {stage.stageType.type == "deploy" && (
                <div className="ml-4">
                  <div className="flex flex-row">
                    {stage.stageType.hostGroup.name}
                    <div className="ml-2 text-sm self-center">
                      (<i>{stage.stageType.hostGroup.environment}</i>)
                    </div>
                  </div>

                  <div className="ml-4">
                    {stage.stageType.hostGroup.hosts.length > 0 ? (
                      stage.stageType.hostGroup.hosts.map((host, i: number) => (
                        <div key={i}>
                          <div>
                            host: <code>{host.name}</code> ({host.arch})
                          </div>
                        </div>
                      ))
                    ) : (
                      <div>No hosts!</div>
                    )}
                  </div>
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
