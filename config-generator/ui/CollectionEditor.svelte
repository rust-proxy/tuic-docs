<script lang="ts">
  import Field from './Field.svelte';
  import type { CollectionView, Dispatch } from './types';
  let { collection, dispatch }: { collection: CollectionView; dispatch: Dispatch } = $props();
</script>

<div hidden={!collection.visible}>
  <h3>{collection.label}</h3>
  {#each collection.rows as row (row.id)}
    <div class="cg-user" hidden={!row.visible}>
      <div class="cg-row-title">
        <strong>{collection.label} {row.number}</strong>
        <button type="button" hidden={!collection.generated}
          onclick={() => dispatch({ type: 'generate-row', collection: collection.name, id: row.id })}>
          {collection.generate_label}
        </button>
        <button type="button" hidden={!collection.removable} aria-label={`移除${collection.label} ${row.number}`}
          onclick={() => dispatch({ type: 'remove', collection: collection.name, id: row.id })}>移除</button>
      </div>
      {#each row.fields as field (field.key)}
        <Field {field} {dispatch} row={{ collection: collection.name, id: row.id }} />
      {/each}
    </div>
  {/each}
  <button type="button" hidden={!collection.editable}
    onclick={() => dispatch({ type: 'add', collection: collection.name })}>{collection.add_label}</button>
  {#if collection.selector}
    <Field field={collection.selector} {dispatch} />
  {/if}
  <p class="cg-hint">{collection.hint}</p>
</div>
