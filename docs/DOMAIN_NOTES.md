# Domain Notes: L5X Structure and Logix Data Types

Working notes for implementers. Primary reference: Rockwell publication
**1756-RM084, *Logix 5000 Controllers Import/Export Reference Manual***, plus
1756-RM003 (*General Instructions*) for instruction operand semantics.
Verify details against the manual version that matches the file's
`SoftwareRevision`. Mark anything learned from a real file with the version
it came from.

## 1. L5X top-level shape

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<RSLogix5000Content SchemaRevision="1.0" SoftwareRevision="33.01"
    TargetName="Line1" TargetType="Controller" ContainsContext="false"
    ExportDate="..." ExportOptions="References NoRawData L5KData DecoratedData ...">
  <Controller Use="Target" Name="Line1" ProcessorType="1756-L85E" MajorRev="33" ...>
    <Description>...</Description>
    <RedundancyInfo/> <Security/> <SafetyInfo/>
    <DataTypes>            <DataType .../>* </DataTypes>
    <Modules>              <Module .../>*   </Modules>
    <AddOnInstructionDefinitions> <AddOnInstructionDefinition .../>* </AddOnInstructionDefinitions>
    <Tags>                 <Tag .../>*      </Tags>          <!-- controller scope -->
    <Programs>             <Program .../>*  </Programs>
    <Tasks>                <Task .../>*     </Tasks>
    <ParameterConnections> <ParameterConnection EndPoint1="\P1.Out" EndPoint2="\P2.In"/>* </ParameterConnections>
    <CST/> <WallClockTime/> <Trends/> <QuickWatchLists/> <TimeSynchronize/> ...
  </Controller>
</RSLogix5000Content>
```

- **Partial exports**: `TargetType` is `Program`, `Routine`, `DataType`,
  `AddOnInstructionDefinition` or `Rung`. With `ContainsContext="true"`, the
  file carries the dependencies (types, AOIs, tags) needed by the target, and
  elements have `Use="Context"` vs `Use="Target"`.
- Element order is stable in practice, but do not rely on it: types may be
  referenced before they are declared.

## 2. Programs and routines

```xml
<Program Name="Filler" MainRoutineName="Main" FaultRoutineName="Fault" Disabled="false" UseAsFolder="false">
  <Tags>...</Tags>                         <!-- program scope; also program parameters (v24+) -->
  <Routines>
    <Routine Name="Main" Type="RLL">       <!-- RLL | ST | FBD | SFC -->
      <RLLContent>
        <Rung Number="0" Type="N">
          <Comment><![CDATA[Start sequence]]></Comment>
          <Text><![CDATA[XIC(Start)[OTL(Run),JSR(Seq,0)];]]></Text>
        </Rung>
      </RLLContent>
    </Routine>
    <Routine Name="Calc" Type="ST">
      <STContent><Line Number="0"><![CDATA[Speed := Ref * 0.5;]]></Line></STContent>
    </Routine>
  </Routines>
</Program>
```

- Rung `Type`: `N` normal; edits-pending variants (`I`, `R`, `D`, `rr`/`ie`
  combinations) exist in online-edit exports. Record and warn, analyse the
  normal text.
- **Execution model**: a Task schedules Programs in order (`ScheduledPrograms`).
  Each Program runs its `MainRoutine` only. Other routines run only when called
  with `JSR` (ladder), `JSR(...)` in ST, or a JSR block in FBD. A routine never
  called is dead code. The fault routine runs on a major fault in that program;
  the controller has a power-up handler and a controller fault handler program.
- `SFR`/`SFP` reset and pause SFC routines; `EVENT` triggers event tasks.

## 3. Ladder neutral text

Grammar outline (to be formalised in `s5k-logic`):

```text
rung        := element* ';'
element     := instruction | branch
branch      := '[' element* (',' element*)* ']'
instruction := MNEMONIC '(' operand (',' operand)* ')'
operand     := tag_path | literal | '?' | expression   (CPT, CMP, FSC take an expression string)
tag_path    := ['\'program_name'.']name ('.' member | '[' index (',' index)* ']')* ('.' bit)?
```

- Operand read/write semantics come from an instruction catalogue, not the
  grammar. Examples: `XIC/XIO` read BOOL; `OTE/OTL/OTU` write BOOL; `MOV` reads
  src, writes dest; `TON/TOF/RTO` read and write a `TIMER`; `JSR` names a
  routine plus input/return parameters.
- AOI calls look like instructions: `MyAoi(MyAoi_Inst, InParam1, OutParam1)`.
  The first operand is the backing tag (instance), followed by required parameters
  in declaration order.
- Bit addressing: `DintTag.5`, `BoolArray[3]`, `DintArray[i].[j]`-style indirect forms exist.

## 4. Data types and how they interact

### 4.1 Categories

| Category | Examples | Notes |
|---|---|---|
| Atomic | `BOOL SINT INT DINT LINT USINT UINT UDINT ULINT REAL LREAL` | Unsigned and 64-bit types depend on controller family/firmware. |
| Predefined structures | `TIMER COUNTER CONTROL STRING MESSAGE PID ALARM_DIGITAL MOTION_INSTRUCTION AXIS_* ...` | Members fixed by firmware (e.g. `TIMER.PRE .ACC .EN .TT .DN`). Ship as a built-in table. |
| Module-defined | `AB:1756_IF8:I:0`, `AB:Embedded_DiscreteIO:O:0` | Come from I/O modules; L5X can contain their definitions. |
| User-defined (UDT) | `<DataType Name="Motor_UDT" Family="NoFamily" Class="User">` | Members can be any type, including other UDTs, AOIs and arrays. |
| String-family UDT | `Family="StringFamily"` | Custom-length strings: `LEN` + `DATA` SINT array. |
| AOI types | each AOI definition is also a data type | Instance tags of AOI type hold parameters and local tags. |

### 4.2 UDT member layout details

```xml
<DataType Name="Valve_UDT" Family="NoFamily" Class="User">
  <Members>
    <Member Name="ZZZZZZZZZZValve_UDT0" DataType="SINT" Dimension="0" Hidden="true"/>
    <Member Name="Open"   DataType="BIT"  Dimension="0" Hidden="false" Target="ZZZZZZZZZZValve_UDT0" BitNumber="0"/>
    <Member Name="Closed" DataType="BIT"  Dimension="0" Hidden="false" Target="ZZZZZZZZZZValve_UDT0" BitNumber="1"/>
    <Member Name="Timer"  DataType="TIMER" Dimension="0"/>
    <Member Name="Hist"   DataType="REAL" Dimension="10"/>
  </Members>
</DataType>
```

- Consecutive BOOL members are packed into hidden `SINT` host members; in L5X
  they appear as `DataType="BIT"` with `Target` + `BitNumber`. The model should
  present them as BOOL members and hide the host.
- `Dimension="0"` means scalar. UDT members support one dimension; tags support
  up to three (`Dimensions="10 5 2"`).
- **Dependency graph**: a UDT depends on every non-atomic member type. Studio
  5000 forbids cycles, but the analyser still checks (corrupt/hand-edited files).

### 4.3 AOI definitions

- Parameters: `Usage` = `Input` | `Output` | `InOut`; `Required`, `Visible`.
  `EnableIn`/`EnableOut` are implicit BOOL parameters.
- Local tags are private to the AOI.
- Routines: `Logic` (required), optional `Prescan`, `Postscan`, `EnableInFalse`.
- Inside AOI logic, names resolve to parameters and local tags only. There is
  no access to controller or program tags except through InOut parameters.
- `InOut` parameters are references: a write inside the AOI is a write to the
  caller's argument tag. The cross-reference must propagate through them.
- Source-protected AOIs: logic replaced by `<EncodedData>`; parameters remain
  visible. Treat as a black box with read/write effects inferred from usage.
- AOIs can call other AOIs (nested), but not recursively.

### 4.4 Tags

- `TagType`: `Base`, `Alias` (`AliasFor="Other.Member"`), `Produced`, `Consumed`.
- Scope: controller tags are global. Program tags are visible inside that
  program and shadow controller tags with the same name. Since v24, program
  parameters (`Usage="Input|Output|InOut|Public"`) can be read across
  programs using `\Program.Tag` syntax.
- `Constant="true"` tags can't be written by logic; a write is a diagnostic.
- `ExternalAccess` (`Read/Write`, `Read Only`, `None`) matters for HMI/SCADA
  interface documentation.

### 4.5 Interaction summary

```text
Task ──schedules──► Program ──main──► Routine ──JSR──► Routine
                       │                 │
                       │ owns            │ references (read/write)
                       ▼                 ▼
                   Program Tags ──type──► DataType (atomic | predefined | module | UDT | AOI)
Controller Tags ──type──────────────────►     │
                                               │ members (recursive)
AOI Definition ──is a──► DataType               ▼
      │ params / locals                   DataType ...
      └─ Logic routine (scoped to the AOI)
Module ──defines──► module-defined types + I/O tags (Local:slot:I/O/C)
```

## 5. FBD and SFC (structure only)

- **FBD**: `<FBDContent SheetSize=...><Sheet Number="1">` with `IRef`, `ORef`,
  `ICon`/`OCon` (connectors across sheets), `Block` (built-in function),
  `AddOnInstruction` blocks, `JSR`, and `Wire FromID FromParam ToID ToParam`.
  Execution order follows data flow, with feedback wires breaking cycles.
- **SFC**: `<SFCContent>` with `Step` (`InitialStep="true"`), `Transition`
  (condition in ST or ladder text), `Branch` (`BranchType="Simultaneous|Selection"`,
  `BranchFlow="Converge|Diverge"`), `DirectedLink FromID ToID`, `Action`s
  with qualifiers (N, P, S, R, L, D ...), `Stop` elements.
  This maps almost one-to-one to a state chart.

## 6. Version matrix (fill in as fixtures are collected)

| SoftwareRevision | Source | Notes |
|---|---|---|
| 32.02, 33.01 | Synthetic fixtures only | Real-file confirmation still needed (OPEN_QUESTIONS A5) |
