import { useState, useEffect } from "react";

export type Result<T, E> = { loading: boolean; res?: T; err?: E };
export type Response<T> = { success: boolean; data?: T };

export function useQuery<T, E>(
  fn: () => Promise<{ success: boolean; data?: T }>
): Result<T, E> {
  const [res, setRes] = useState<T>();
  const [err, setErr] = useState<E>();
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function fetchData() {
      try {
        const res = await fn();
        console.log("GOT RES: ", res);
        if (res.success) setRes(res.data);
        else setErr(res as E);
      } catch (e) {
        console.error(e);
        setErr(e as E);
      } finally {
        setLoading(false);
      }
    }

    fetchData();
  }, [setErr, setRes, setLoading, fn]);

  return { res, err, loading };
}
