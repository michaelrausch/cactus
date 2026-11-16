import { useState } from "react";
import { Copy, FolderPen, Trash2 } from "lucide-react";
import { deleteCritter, duplicateCritter, renameCritterFolder, updateCritter } from "@/actions/critters";
import type { CritterSummary, Species } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { SpeciesGrid } from "../SpeciesGrid";

/** The Select can't hold an empty value, so "no login flow" has a stand-in. */
const NO_LOGIN = "__none__";

/**
 * Species, notes and login flow, each saved to `critter.yml` as it changes (notes on blur),
 * like the app's other settings. The name is edited in the page header. Mount with a `key` per critter.
 */
export function CritterSettingsTab({ critter }: { critter: CritterSummary }) {
  const [notes, setNotes] = useState(critter.notes);

  const setSpecies = (species: Species) => {
    if (species !== critter.species) void updateCritter(critter.id, (saved) => ({ ...saved, species }));
  };
  const saveNotes = () => {
    if (notes !== critter.notes) void updateCritter(critter.id, (saved) => ({ ...saved, notes }));
  };
  const setLogin = (login: string) => {
    if (login === (critter.login ?? NO_LOGIN)) return;
    void updateCritter(critter.id, ({ login: _previous, ...rest }) => ({
      ...rest,
      // Left out, the key is removed from critter.yml.
      ...(login === NO_LOGIN ? {} : { login }),
    }));
  };

  return (
    <div className="flex max-w-xl flex-col gap-8 p-5">
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium">Species</span>
          <SpeciesGrid value={critter.species} onChange={setSpecies} />
        </div>
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium">Notes</span>
          <Textarea
            value={notes}
            rows={2}
            placeholder="e.g. Viewer account on staging"
            onChange={(event) => setNotes(event.target.value)}
            onBlur={saveNotes}
          />
        </label>
        <div className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium">Login flow</span>
          <Select value={critter.login ?? NO_LOGIN} onValueChange={setLogin}>
            <SelectTrigger className="w-64">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={NO_LOGIN}>None</SelectItem>
              {critter.flows.map((flow) => (
                <SelectItem key={flow.id} value={flow.id}>
                  {flow.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      <section className="flex flex-col gap-2 text-sm">
        <h3 className="font-medium">Folder</h3>
        <div className="flex items-center gap-2">
          <span className="font-mono text-xs text-muted-foreground">critters/{critter.id}/</span>
          <Button variant="ghost" size="sm" onClick={() => renameCritterFolder(critter.id)}>
            <FolderPen /> Rename…
          </Button>
        </div>
      </section>

      <section className="flex flex-wrap gap-2 border-t pt-5">
        <Button variant="outline" onClick={() => duplicateCritter(critter.id)}>
          <Copy /> Duplicate…
        </Button>
        <Button variant="ghost" className="hover:text-danger" onClick={() => deleteCritter(critter.id)}>
          <Trash2 /> Delete…
        </Button>
      </section>
    </div>
  );
}
