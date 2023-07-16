"use client";

import { errorOr } from "@/components/UnhappyPath";
import { getUnseenAnts } from "@/server/queries";
import { useQuery } from "@tanstack/react-query";
import React, { useState } from "react";

function formatDate(createdUtcMilliseconds: string): string {
  const d = new Date(createdUtcMilliseconds);
  return `${d.toLocaleDateString()} ${d.toLocaleTimeString()}`;
}

export default function Feed() {
  const [page, setPage] = useState(0);

  const { isLoading, isError, data, refetch } = useQuery({
    queryKey: ["unseenAnts"],
    queryFn: () => getUnseenAnts(page),
  });

  return errorOr(isLoading, isError, { unseenAnts: data }, ({ unseenAnts }) => (
    <div>
      <h3>
        latest ant submissions ({unseenAnts.length}):{" "}
        <button onClick={() => refetch()}>refresh</button>
      </h3>
      {unseenAnts.map((ant, i) => (
        <div key={ant.ant_name + i}>
          [{formatDate(ant.created_at)}] <strong>{ant.ant_name}</strong>
        </div>
      ))}
    </div>
  ));
}
