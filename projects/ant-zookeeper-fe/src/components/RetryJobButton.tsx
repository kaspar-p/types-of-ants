"use client";

export type RetryJobButtonProps = {
  jobId: string;
};

export function RetryJobButton(props: RetryJobButtonProps) {
  return (
    <button
      onClick={() => {
        fetch("http://localhost:3235/deployment/retry", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            jobId: props.jobId,
          }),
        });
      }}
    >
      retry?
    </button>
  );
}
