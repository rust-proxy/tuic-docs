<script lang="ts">
  import type { Dispatch, FieldView } from './types';

  let { field, dispatch, row }: { field: FieldView; dispatch: Dispatch; row?: { collection: string; id: string } } = $props();
  const id = $derived(`cg-${field.path}`);
  const described = $derived(`${id}-hint ${id}-error`);

  function change(value: string) {
    dispatch(row ? { type: 'set-row', ...row, field: field.key, value } : { type: 'set', field: field.key, value });
  }
</script>

<div class:cg-toggle={field.kind === 'toggle'} class="cg-field" hidden={!field.visible}>
  <label for={id}>{field.label}</label>
  {#if field.kind === 'toggle'}
    <input {id} type="checkbox" checked={field.value === 'true'}
      aria-describedby={described} aria-invalid={!!field.error}
      onchange={(event) => change(String(event.currentTarget.checked))} />
  {:else if field.kind === 'select'}
    <select {id} value={field.value} aria-describedby={described} aria-invalid={!!field.error}
      onchange={(event) => change(event.currentTarget.value)}>
      {#each field.options as [value, label] (value)}
        <option {value}>{label}</option>
      {/each}
    </select>
  {:else}
    <input {id} type={field.kind === 'number' ? 'text' : field.kind}
      value={field.value} placeholder={field.placeholder} autocomplete="off" spellcheck="false"
      inputmode={field.kind === 'number' ? 'numeric' : undefined}
      aria-describedby={described} aria-invalid={!!field.error}
      oninput={(event) => change(event.currentTarget.value)} />
  {/if}
  <span id={`${id}-hint`} class="cg-hint">{field.hint}</span>
  <span id={`${id}-error`} class="cg-error">{field.error}</span>
  {#if !row && field.generated}
    <button type="button" onclick={() => dispatch({ type: 'generate', field: field.key })}>生成随机值</button>
  {/if}
</div>
