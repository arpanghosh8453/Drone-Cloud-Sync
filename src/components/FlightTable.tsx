import { useMemo, useState } from "react";
import {
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  useReactTable,
  type ColumnDef,
  type RowSelectionState,
  type SortingState,
} from "@tanstack/react-table";
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  CheckCheck,
  ChevronRight,
  FileText,
  Search,
  X,
  Download,
} from "lucide-react";
import type { FlightRecord } from "../types";
import { date, value } from "../lib/utils";
import { Checkbox } from "./ui/checkbox";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
interface Props {
  records: FlightRecord[];
  onOpen: (r: FlightRecord) => void;
  onDownload: (ids: string[]) => void;
  canDownload: boolean;
  preferences?: string;
  savePreferences: (p: string) => void;
}
export function FlightTable({
  records,
  onOpen,
  onDownload,
  canDownload,
  preferences,
  savePreferences,
}: Props) {
  const initial = useMemo(() => {
    try {
      return JSON.parse(preferences || "{}") as {
        search?: string;
        aircraft?: string;
        status?: string;
        sorting?: SortingState;
      };
    } catch {
      return {};
    }
  }, []);
  const [search, setSearch] = useState(initial.search || "");
  const [aircraft, setAircraft] = useState(initial.aircraft || "");
  const [status, setStatus] = useState(initial.status || "");
  const [sorting, setSorting] = useState<SortingState>(
    initial.sorting || [{ id: "sortTimestamp", desc: true }],
  );
  const [selection, setSelection] = useState<RowSelectionState>({});
  function persist(changes: object) {
    savePreferences(
      JSON.stringify({ search, aircraft, status, sorting, ...changes }),
    );
  }
  const columns = useMemo<ColumnDef<FlightRecord>[]>(
    () => [
      {
        id: "select",
        enableSorting: false,
        header: ({ table }) => (
          <Checkbox
            aria-label="Select all matching records"
            checked={
              table.getIsAllRowsSelected()
                ? true
                : table.getIsSomeRowsSelected()
                  ? "indeterminate"
                  : false
            }
            onCheckedChange={(v) => table.toggleAllRowsSelected(v === true)}
          />
        ),
        cell: ({ row }) => (
          <div onClick={(e) => e.stopPropagation()}>
            <Checkbox
              aria-label={`Select ${row.original.originalFilename}`}
              checked={row.getIsSelected()}
              onCheckedChange={(v) => row.toggleSelected(v === true)}
            />
          </div>
        ),
      },
      {
        accessorKey: "sortTimestamp",
        header: "Date",
        cell: ({ row }) => (
          <span className="date-cell">
            {date(row.original.flightTimestamp, row.original.sortTimestamp)}
          </span>
        ),
      },
      {
        accessorKey: "aircraftName",
        header: "Aircraft",
        cell: (c) => <span>{value(c.getValue())}</span>,
      },
      {
        accessorKey: "duration",
        header: "Duration",
        cell: (c) => <span className="tabular">{value(c.getValue())}</span>,
        sortingFn: (a, b, id) => numeric(a.getValue(id), b.getValue(id)),
      },
      {
        accessorKey: "distance",
        header: "Distance",
        cell: (c) => <span className="tabular">{value(c.getValue())}</span>,
        sortingFn: (a, b, id) => numeric(a.getValue(id), b.getValue(id)),
      },
      {
        accessorKey: "maxAltitude",
        header: "Altitude",
        cell: (c) => <span className="tabular">{value(c.getValue())}</span>,
        sortingFn: (a, b, id) => numeric(a.getValue(id), b.getValue(id)),
      },
      {
        accessorKey: "serverMd5",
        header: "Hash",
        enableSorting: false,
        cell: (c) => (
          <code className="hash-short" title={value(c.getValue())}>
            {typeof c.getValue() === "string"
              ? String(c.getValue()).slice(0, 8) + "…"
              : "—"}
          </code>
        ),
      },
      {
        accessorKey: "status",
        header: "Status",
        enableSorting: false,
        cell: (c) => (
          <Badge
            tone={
              c.getValue() === "Downloaded"
                ? "success"
                : c.getValue() === "Failed"
                  ? "danger"
                  : "neutral"
            }
          >
            {String(c.getValue())}
          </Badge>
        ),
        filterFn: "equalsString",
      },
      {
        accessorKey: "originalFilename",
        header: "Filename",
        cell: (c) => (
          <span className="filename-cell" title={String(c.getValue())}>
            <FileText size={14} />
            {String(c.getValue())}
          </span>
        ),
      },
      {
        id: "details",
        header: "",
        enableSorting: false,
        cell: () => <ChevronRight size={15} className="muted" />,
      },
    ],
    [],
  );
  const columnFilters = useMemo(
    () => [
      ...(search ? [{ id: "originalFilename", value: search }] : []),
      ...(aircraft ? [{ id: "aircraftName", value: aircraft }] : []),
      ...(status ? [{ id: "status", value: status }] : []),
    ],
    [search, aircraft, status],
  );
  const table = useReactTable({
    data: records,
    columns,
    getRowId: (r) => r.id,
    state: { sorting, rowSelection: selection, columnFilters },
    onSortingChange: (updater) => {
      const next = typeof updater === "function" ? updater(sorting) : updater;
      setSorting(next);
      persist({ sorting: next });
    },
    onRowSelectionChange: setSelection,
    getCoreRowModel: getCoreRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    initialState: { pagination: { pageIndex: 0, pageSize: 100 } },
    autoResetPageIndex: true,
    enableRowSelection: true,
  });
  const selected = records.filter((r) => selection[r.id]).map((r) => r.id);
  const filtered = table.getFilteredRowModel().rows.length;
  return (
    <div className="flight-table-wrap">
      <div className="filter-toolbar">
        <div className="search-field">
          <Search size={16} />
          <Input
            aria-label="Search filenames"
            placeholder="Search filenames…"
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              persist({ search: e.target.value });
            }}
          />
        </div>
        <select
          aria-label="Filter by aircraft"
          value={aircraft}
          onChange={(e) => {
            setAircraft(e.target.value);
            persist({ aircraft: e.target.value });
          }}
        >
          <option value="">All aircraft</option>
          {Array.from(
            new Set(
              records
                .map((r) => r.aircraftName)
                .filter((v): v is string => !!v),
            ),
          )
            .sort()
            .map((a) => (
              <option key={a}>{a}</option>
            ))}
        </select>
        <select
          aria-label="Filter by download status"
          value={status}
          onChange={(e) => {
            setStatus(e.target.value);
            persist({ status: e.target.value });
          }}
        >
          <option value="">All statuses</option>
          {["Downloaded", "Missing", "Downloading", "Failed"].map((s) => (
            <option key={s}>{s}</option>
          ))}
        </select>
        {(search || aircraft || status) && (
          <Button
            size="icon"
            variant="ghost"
            aria-label="Clear filters"
            title="Clear filters"
            onClick={() => {
              setSearch("");
              setAircraft("");
              setStatus("");
              persist({ search: "", aircraft: "", status: "" });
            }}
          >
            <X size={16} />
          </Button>
        )}
        <span className="filter-count">{filtered} records</span>
      </div>
      <div className="selection-actions">
        <Button
          size="sm"
          variant="ghost"
          onClick={() =>
            setSelection(Object.fromEntries(records.map((r) => [r.id, true])))
          }
        >
          <CheckCheck size={14} />
          Select all records
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => table.toggleAllRowsSelected(true)}
        >
          Select matching
        </Button>
        <span>Values are shown as supplied by the cloud; units may vary.</span>
      </div>
      {selected.length > 0 && (
        <div className="selection-bar">
          <strong>{selected.length} selected</strong>
          <Button
            size="sm"
            disabled={!canDownload}
            onClick={() => onDownload(selected)}
          >
            <Download size={14} />
            Download selected
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setSelection({})}>
            Clear selection
          </Button>
        </div>
      )}
      <div className="table-scroll">
        <table>
          <thead>
            {table.getHeaderGroups().map((g) => (
              <tr key={g.id}>
                {g.headers.map((h) => (
                  <th key={h.id}>
                    {h.column.getCanSort() ? (
                      <button onClick={h.column.getToggleSortingHandler()}>
                        {flexRender(h.column.columnDef.header, h.getContext())}
                        {h.column.getIsSorted() === "desc" ? (
                          <ArrowDown size={12} />
                        ) : h.column.getIsSorted() ? (
                          <ArrowUp size={12} />
                        ) : (
                          <ArrowUpDown size={12} />
                        )}
                      </button>
                    ) : (
                      flexRender(h.column.columnDef.header, h.getContext())
                    )}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <tr
                key={row.id}
                className={row.getIsSelected() ? "selected" : ""}
                tabIndex={0}
                aria-label={`View ${row.original.originalFilename}`}
                onClick={() => onOpen(row.original)}
                onKeyDown={(e) => {
                  if (
                    e.target === e.currentTarget &&
                    (e.key === "Enter" || e.key === " ")
                  ) {
                    e.preventDefault();
                    onOpen(row.original);
                  }
                }}
              >
                {row.getVisibleCells().map((c) => (
                  <td key={c.id}>
                    {flexRender(c.column.columnDef.cell, c.getContext())}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
        {filtered === 0 && (
          <div className="empty-table">
            <FileText size={28} />
            <h3>
              {records.length
                ? "No matching flights"
                : "Your flight records will appear here"}
            </h3>
            <p>
              {records.length
                ? "Try another filename or clear the filters."
                : "Refresh the cloud inventory to retrieve your flight logs."}
            </p>
          </div>
        )}
      </div>
      <div className="table-footer">
        <span>
          {selected.length} of {records.length} selected
        </span>
        <div className="table-pages">
          <span>
            {filtered ? table.getState().pagination.pageIndex * 100 + 1 : 0}–
            {Math.min(
              (table.getState().pagination.pageIndex + 1) * 100,
              filtered,
            )}{" "}
            of {filtered}
          </span>
          <Button
            size="sm"
            variant="ghost"
            aria-label="Previous records page"
            disabled={!table.getCanPreviousPage()}
            onClick={() => table.previousPage()}
          >
            Previous
          </Button>
          <Button
            size="sm"
            variant="ghost"
            aria-label="Next records page"
            disabled={!table.getCanNextPage()}
            onClick={() => table.nextPage()}
          >
            Next
          </Button>
        </div>
      </div>
    </div>
  );
}
function numeric(a: unknown, b: unknown) {
  const av = Number(a),
    bv = Number(b);
  return Number.isNaN(av) || Number.isNaN(bv)
    ? value(a).localeCompare(value(b))
    : av - bv;
}
