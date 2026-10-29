p = 'ui/src/lib/components/studio/designers/page/IconPicker.svelte'
c = open(p, 'r', encoding='utf-8').read()
old = '      type="button"\n      class={"group flex min-w-0'
new = '      type="button"\n      data-testid="icon-picker-trigger"\n      class={"group flex min-w-0'
assert old in c, 'not found'
open(p, 'w', encoding='utf-8').write(c.replace(old, new, 1))
print('Added data-testid')
