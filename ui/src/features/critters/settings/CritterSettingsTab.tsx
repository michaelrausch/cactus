import { useState } from "react";
import { Copy, FolderPen, Trash2 } from "lucide-react";
import { deleteCritter, duplicateCritter, renameCritterFolder, updateCritter } from "@/actions/critters";
import type { CritterSummary, Species } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { critterNameProblem } from "@/domain/critterId";
import { useCritterStore } from "@/stores/critterStore";
import { SpeciesGrid } from "../SpeciesGrid";

/** The Select can't hold an empty value, so "no login flow" has a stand-in. */
const NO_LOGIN = "__none__";

/** Name, species, notes and login flow, saved to `critter.yml` in place. Mount with a `key` per critter. */
export function CritterSettingsTab({ critter }: { critter: CritterSummary }) {
  const critters = useCritterStore((s) => s.critters);
  const others = critters.filter((c) => c.id !== critter.id);
  const [name, setName] = useState(critter.name);
  const [species, setSpecies] = useState<Species>(critter.species);
  const [notes, setNotes] = useState(critter.notes);
  const [login, setLogin] = useState(critter.login ?? NO_LOGIN);
  const [saving, setSaving] = useState(false);

  const problem = critterNameProblem(name, others);
  const dirty =
    name.trim() !== critter.name ||
    species !== critter.species ||
    notes !== critter.notes ||
    login !== (critter.login ?? NO_LOGIN);

  const save = async () => {
    if (problem || !dirty) return;
    setSaving(true);
    await updateCritter(critter.id, ({ login: _previous, ...rest }) => ({
      ...rest,
      name: name.trim(),
      species,
      notes,
      // Left out, the key is removed from critter.yml.
      ...(login === NO_LOGIN ? {} : { login }),
    }));
    setSaving(false);
  };

  return (
    <div className="flex max-w-xl flex-col gap-8 p-5">
      <form
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault();
          void save();
        }}
      >
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium">Name</span>
          <Input value={name} onChange={(event) => setName(event.target.value)} aria-invalid={!!problem} />
          {problem && <span className="text-xs text-danger">{problem}</span>}
        </label>
        <div className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium">Species</span>
          <SpeciesGrid value={species} onChange={setSpecies} />
        </div>
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium">Notes</span>
          <Textarea
            value={notes}
            rows={2}
            placeholder="e.g. Viewer account on staging"
            onChange={(event) => setNotes(event.target.value)}
          />
        </label>
        <div className="flex flex-col gap-1.5 text-sm">
          <span className="font-medium">Login flow</span>
          <Select value={login} onValueChange={setLogin}>
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
        <Button type="submit" className="self-start" disabled={!dirty || !!problem || saving}>
          Save
        </Button>
      </form>

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
