import { createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { useClient } from "@/hooks/use-client";

export const Route = createFileRoute("/_app/")({
  component: OverviewPage,
});

function OverviewPage() {
  const client = useClient();
  const statisticsQuery = useQuery({
    queryKey: ["store-statistics", client],
    enabled: client !== null,
    queryFn: async () => {
      if (client === null) {
        throw new Error("API client is unavailable");
      }

      const result = await client.GET("/store/statistics");
      if (result.error !== undefined || result.data === undefined) {
        throw new Error("Unable to load store statistics.");
      }

      return result.data;
    },
  });

  const statistics = statisticsQuery.data ?? null;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Overview</h1>
        <p className="text-muted-foreground mt-1">
          Current translation store statistics.
        </p>
      </div>

      {statisticsQuery.isError ? (
        <Card>
          <CardHeader>
            <CardTitle>Statistics unavailable</CardTitle>
            <CardDescription>
              {statisticsQuery.error instanceof Error
                ? statisticsQuery.error.message
                : "Unable to load store statistics."}
            </CardDescription>
          </CardHeader>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-3">
          <StatisticCard
            label="Locales"
            value={statistics?.locales.toLocaleString() ?? "—"}
          />
          <StatisticCard
            label="Translations"
            value={statistics?.entries.toLocaleString() ?? "—"}
          />
          <StatisticCard
            label="Active store"
            value={statistics === null ? "Loading" : "Connected"}
          />
        </div>
      )}

      {statistics !== null && (
        <Card>
          <CardHeader>
            <CardTitle>Entries by locale</CardTitle>
            <CardDescription>
              Number of translation entries currently stored for each locale.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="divide-y rounded-md border">
              {Object.entries(statistics.per_locale).map(
                ([locale, entries]) => (
                  <div
                    className="flex items-center justify-between px-4 py-3"
                    key={locale}
                  >
                    <span className="font-medium">{locale}</span>
                    <span className="text-muted-foreground text-sm">
                      {entries.toLocaleString()} entries
                    </span>
                  </div>
                ),
              )}
              {Object.keys(statistics.per_locale).length === 0 && (
                <p className="text-muted-foreground px-4 py-6 text-sm">
                  No translation entries yet.
                </p>
              )}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

type StatisticCardProps = {
  readonly label: string;
  readonly value: string;
};

function StatisticCard({ label, value }: StatisticCardProps) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardDescription>{label}</CardDescription>
        <CardTitle className="text-3xl">{value}</CardTitle>
      </CardHeader>
    </Card>
  );
}
