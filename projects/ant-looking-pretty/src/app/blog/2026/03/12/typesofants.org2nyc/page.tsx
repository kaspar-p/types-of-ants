import { BlogPage } from "@/app/blog/BlogPage";

export default function NycMoveBlog() {
  return (
    <BlogPage
      title="typesofants.org moves to nyc"
      href="/blog/2026/03/12/typesofants.org2nyc"
      formattedDate={"Feb 28 2026 - Mar 12 2026"}
    >
      <div>
        this is an event log kept daily during typesofants.org's move from
        toronto, ontario, to nyc.
      </div>

      <div className="mb-1">
        <strong>what happened?</strong>
      </div>

      <div className="flex flex-col space-y-1">
        <div className="pl-3">
          <strong>2026-02-28</strong>: typesofants.org was taken offline
        </div>

        <div className="pl-3">
          <strong>2026-03-01</strong>: typesofants.org started the move in fort
          collins, colorado
        </div>

        <div className="pl-3">
          <strong>2026-03-02</strong>: typesofants.org made it to omaha,
          nebraska wayy too late
        </div>

        <div className="pl-3">
          <strong>2026-03-03</strong>: typesofants.org experienced the busiest
          culver's in america in iowa city, iowa
        </div>

        <div className="pl-3">
          <strong>2026-03-03</strong>: typesofants.org spent the night in
          kalamazoo, michigan
        </div>

        <div className="pl-3">
          <strong>2026-03-04</strong>: typesofants.org made it safely to
          toronto, ontario
        </div>

        <div className="pl-3">
          <strong>2026-03-05</strong>: typesofants.org spent the day packing and
          stressing about how much stuff it owns in toronto, ontario
        </div>

        <div className="pl-3">
          <strong>2026-03-06</strong>: typesofants.org experienced the windiest,
          hilliest, foggiest roads to spend the night in lincoln park, new
          jersey
        </div>

        <div className="pl-3">
          <strong>2026-03-07</strong>: typesofants.org enlisted someone's help
          and moved everything into its new home in brooklyn, ny! it slept
          dog-tired
        </div>

        <div className="pl-3">
          <strong>2026-03-08</strong>: typesofants.org explored and shopped for
          its first new groceries, furniture, and built its eventual home desk
        </div>

        <div className="pl-3">
          <strong>2026-03-09</strong>: typesofants.org had sushi on the floor
        </div>

        <div className="pl-3">
          <strong>2026-03-10</strong>: typesofants.org plugged in all (8)
          servers, (2) network switches, (4) power blocks, (1) extension cord,
          (16) nails and a complete lack of cable organization. still no
          internet
        </div>

        <div className="pl-3">
          <strong>2026-03-12</strong>: typesofants.org finally got internet
          installed and came online in the form of 3 servers plugged together on
          the floor, after a bit of fighting with verizon
        </div>
      </div>
    </BlogPage>
  );
}
