import type { HttpResponse } from "@/api/client";

export function HeadersTable({ response }: { response: HttpResponse }) {
  return (
    <>
      <div className="mb-3 font-mono text-xs break-all text-muted-foreground">
        {response.method} {response.url}
      </div>
      <table className="w-full font-mono text-xs">
        <tbody>
          {response.headers.map(([name, value], index) => (
            // oxlint-disable-next-line react/no-array-index-key -- headers may repeat (Set-Cookie) and never reorder
            <tr key={index} className="border-b border-border/50">
              <td className="py-1.5 pr-4 align-top whitespace-nowrap text-muted-foreground">{name}</td>
              <td className="py-1.5 break-all select-text">{value}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
}
