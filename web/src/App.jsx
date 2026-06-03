import { useState, useEffect } from "react";

const POLL_INTERVAL_MS = 60_000;

function stateLabel(state) {
  if (!state) return "—";
  switch (state.kind) {
    case "good":
      return "Good";
    case "unresponsive":
      return `Unresponsive (${state.retries})`;
    case "bad":
      return "Bad";
    default:
      return state.kind;
  }
}

function stateClass(state) {
  if (!state) return "text-gray-400";
  switch (state.kind) {
    case "good":
      return "text-green-400";
    case "unresponsive":
      return "text-yellow-400";
    case "bad":
      return "text-red-400";
    default:
      return "text-gray-400";
  }
}

function App() {
  const [makers, setMakers] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [lastUpdated, setLastUpdated] = useState(null);

  const EXPLORER_BASE =
    import.meta.env.VITE_EXPLORER_BASE || "https://mempool.space/signet";

  const fetchMakers = () => {
    fetch("/api/makers")
      .then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return res.json();
      })
      .then((data) => {
        setMakers(data);
        setError(null);
        if (data.length > 0) {
          const latest = Math.max(...data.map((m) => m.timestamp * 1000));
          setLastUpdated(new Date(latest));
        }
        setLoading(false);
      })
      .catch((err) => {
        console.error("Error loading makers:", err);
        setError(err.message);
        setLoading(false);
      });
  };

  useEffect(() => {
    fetchMakers();
    const id = setInterval(fetchMakers, POLL_INTERVAL_MS);
    return () => clearInterval(id);
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-gray-900">
        <div className="flex flex-col items-center gap-4">
          <div className="w-12 h-12 border-4 border-orange-500 border-t-transparent rounded-full animate-spin"></div>
          <span className="text-orange-400 text-xl font-medium">
            Loading Market Data...
          </span>
        </div>
      </div>
    );
  }

  const withOffer = makers.filter((m) => m.offer).length;

  return (
    <div className="min-h-screen bg-gray-900 text-white">
      <div className="max-w-screen-2xl mx-auto p-6">
        <div className="mb-8">
          <h1 className="text-5xl font-bold text-orange-500 mb-2">
            Coinswap Market
          </h1>
          <p className="text-gray-400 text-lg">
            All makers tracked by the daemon — including bad and unresponsive
            ones.
          </p>
        </div>

        {error && (
          <div className="mb-6 p-4 bg-red-900 border border-red-700 rounded-lg text-red-300 text-sm">
            Failed to load makers: {error}
          </div>
        )}

        <div className="bg-gray-800 rounded-xl shadow-2xl border border-gray-700 overflow-hidden">
          <div className="overflow-x-auto">
            <table className="min-w-full">
              <thead>
                <tr className="bg-linear-to-r from-orange-600 to-orange-500">
                  {[
                    { label: "Address", desc: "Maker Address" },
                    { label: "State", desc: "Connection Status" },
                    { label: "Protocol", desc: "Legacy / Taproot" },
                    { label: "Base Fee", desc: "Fixed Fee (sat)" },
                    { label: "Amount", desc: "Volume Fee" },
                    { label: "Time", desc: "Time Fee" },
                    { label: "Min Size", desc: "Minimum Order (sat)" },
                    { label: "Max Size", desc: "Maximum Order (sat)" },
                    { label: "Bond", desc: "Fidelity Bond" },
                  ].map((header, idx) => (
                    <th key={idx} className="px-6 py-4 text-left">
                      <div className="flex flex-col">
                        <span className="text-white font-bold">
                          {header.label}
                        </span>
                        <span className="text-orange-100 text-xs font-normal">
                          {header.desc}
                        </span>
                      </div>
                    </th>
                  ))}
                </tr>
              </thead>

              <tbody className="divide-y divide-gray-700">
                {makers.length === 0 ? (
                  <tr>
                    <td
                      colSpan={9}
                      className="px-6 py-12 text-center text-gray-400"
                    >
                      No makers known yet. The daemon may still be syncing.
                    </td>
                  </tr>
                ) : (
                  makers.map((m) => {
                    const offer = m.offer;
                    const bond = offer?.fidelity_bond;
                    const key = bond
                      ? `${bond.outpoint.txid}:${bond.outpoint.vout}`
                      : m.address;
                    return (
                      <tr
                        key={key}
                        className="hover:bg-gray-700 transition-colors duration-150"
                      >
                        <td className="px-6 py-4">
                          <div className="font-mono text-sm text-orange-300 truncate max-w-xs">
                            {m.address}
                          </div>
                        </td>
                        <td className="px-6 py-4">
                          <span
                            className={`font-medium ${stateClass(m.state)}`}
                          >
                            {stateLabel(m.state)}
                          </span>
                        </td>
                        <td className="px-6 py-4 text-gray-300">
                          {m.protocol ?? "—"}
                        </td>
                        <td className="px-6 py-4">
                          {offer ? (
                            <span className="font-medium text-green-400">
                              {offer.base_fee.toLocaleString()}
                            </span>
                          ) : (
                            <span className="text-gray-500">—</span>
                          )}
                        </td>
                        <td className="px-6 py-4">
                          {offer ? (
                            <span className="text-blue-400 font-medium">
                              {(offer.amount_relative_fee_pct * 100).toFixed(2)}
                              %
                            </span>
                          ) : (
                            <span className="text-gray-500">—</span>
                          )}
                        </td>
                        <td className="px-6 py-4">
                          {offer ? (
                            <span className="text-blue-400 font-medium">
                              {(offer.time_relative_fee_pct * 100).toFixed(2)}%
                            </span>
                          ) : (
                            <span className="text-gray-500">—</span>
                          )}
                        </td>
                        <td className="px-6 py-4">
                          {offer ? (
                            <span className="text-yellow-400">
                              {offer.min_size.toLocaleString()}
                            </span>
                          ) : (
                            <span className="text-gray-500">—</span>
                          )}
                        </td>
                        <td className="px-6 py-4">
                          {offer ? (
                            <span className="text-yellow-400">
                              {offer.max_size.toLocaleString()}
                            </span>
                          ) : (
                            <span className="text-gray-500">—</span>
                          )}
                        </td>
                        <td className="px-6 py-4">
                          {bond ? (
                            <>
                              <div className="font-medium text-gray-200">
                                {bond.amount.toLocaleString()} sat
                              </div>
                              <a
                                href={`${EXPLORER_BASE}/tx/${bond.outpoint.txid}`}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-orange-400 text-xs hover:underline truncate block max-w-xs"
                                title={bond.outpoint.txid}
                              >
                                {bond.outpoint.txid}
                              </a>
                            </>
                          ) : (
                            <span className="text-gray-500">—</span>
                          )}
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>
        </div>

        <div className="mt-6 text-center text-gray-400 text-sm">
          <p>
            Showing {makers.length} maker{makers.length !== 1 ? "s" : ""} (
            {withOffer} with offer{withOffer !== 1 ? "s" : ""}) •{" "}
            {lastUpdated
              ? `Updated ${lastUpdated.toLocaleTimeString()}`
              : "Waiting for first sync..."}
          </p>
        </div>
      </div>
    </div>
  );
}

export default App;
