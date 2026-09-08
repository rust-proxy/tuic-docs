import { Engine } from '../pkg/engine';
import type { Action, ExportFile, Snapshot } from './types';

export class Controller {
  private engine = new Engine();
  private selected = '';
  private reveal = false;
  view: Snapshot = $state.raw(this.read());
  status = $state('');

  constructor() {
    try { this.engine.initialize(); }
    catch (error) { this.status = String(error); }
    this.refresh();
  }

  private read(): Snapshot {
    return JSON.parse(this.engine.snapshot(this.selected, this.reveal));
  }

  private refresh() { this.view = this.read(); }

  dispatch = (action: Action) => {
    try {
      this.engine.dispatch(JSON.stringify(action));
      this.status = action.type.startsWith('generate') ? '已更新随机值。' : '';
      this.refresh();
    } catch (error) { this.status = String(error); }
  };

  select = (name: string) => { this.selected = name; this.refresh(); };
  showSecrets = (reveal: boolean) => { this.reveal = reveal; this.refresh(); };
  export(): ExportFile { return JSON.parse(this.engine.export(this.selected)); }
  dispose() { this.engine.free(); }
}
