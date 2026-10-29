path = 'ui/src/lib/components/studio/designers/page/PropertyPanel.svelte'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

target = '''          <StyleEditor
            style={styleValues}
            inherited={isPage ? {} : (compiled?.style ?? {})}
            schema={model.schema}
            onSet={setStyle}
          />'''

replacement = '''          <StyleEditor
            style={styleValues}
            inherited={isPage ? {} : (compiled?.style ?? {})}
            schema={model.schema}
            activeIcon={ownValue("icon") || styleValues?.icon || compiled?.icon || compiled?.style?.icon || null}
            onSet={setStyle}
            onSetIcon={async (iconName) => {
              if (isHeaderTitle || allRows.some((r) => r.name === "icon")) {
                const iconRow = allRows.find((r) => r.name === "icon") || { name: "icon", type: "icon" };
                await set(iconRow, iconName);
              } else {
                await setStyle("icon", iconName);
              }
            }}
          />'''

assert target in content, "target not found in PropertyPanel.svelte"
content = content.replace(target, replacement, 1)

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print("PropertyPanel.svelte updated successfully!")
