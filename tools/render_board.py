"""Static orthographic CAD linework from the user-supplied OBJ export.
Run: uv run --with numpy --with pillow tools/render_board.py input.obj
No Blender, GUI, or model loading in the desktop runtime.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import numpy as np
from PIL import Image

parser=argparse.ArgumentParser()
parser.add_argument('model',type=Path)
parser.add_argument('--output',type=Path,default=Path('desktop/esp_gauge/assets'))
args=parser.parse_args()
raw=gzip.decompress(args.model.read_bytes()) if args.model.suffix=='.gz' else args.model.read_bytes()
vertices=[]; faces=[]; owners=[]; groups=[]; group=''; group_ids={}
for line in raw.decode().splitlines():
    fields=line.split()
    if not fields: continue
    if fields[0]=='v': vertices.append(tuple(map(float,fields[1:4])))
    elif fields[0]=='g':
        group=' '.join(fields[1:]); group_ids.setdefault(group,len(group_ids)+1)
    elif fields[0]=='f':
        ids=[int(f.split('/')[0])-1 for f in fields[1:]]
        for index in range(1,len(ids)-1):
            faces.append((ids[0],ids[index],ids[index+1])); owners.append(group_ids[group]); groups.append(group)
v=np.array(vertices); f=np.array(faces); owners=np.array(owners)
# Model coordinates are in centimetres. Top view: USB left, ESP antenna right.
lo=v[:,:2].min(axis=0)-.12; hi=v[:,:2].max(axis=0)+.12
width=1200; scale=(width-1)/(hi[0]-lo[0]); height=int((hi[1]-lo[1])*scale)+1
screen=np.column_stack(((v[:,0]-lo[0])*scale,(hi[1]-v[:,1])*scale,v[:,2]))
z=np.full((height,width),-np.inf); normals=np.zeros((height,width,3)); material=np.zeros((height,width),dtype=np.int32)
for tri,owner in zip(f,owners):
    p=screen[tri]; normal=np.cross(v[tri[1]]-v[tri[0]],v[tri[2]]-v[tri[0]])
    length=np.linalg.norm(normal)
    if length<1e-12: continue
    normal/=length
    xmin=max(0,int(np.floor(p[:,0].min()))); xmax=min(width-1,int(np.ceil(p[:,0].max())))
    ymin=max(0,int(np.floor(p[:,1].min()))); ymax=min(height-1,int(np.ceil(p[:,1].max())))
    denominator=(p[1,1]-p[2,1])*(p[0,0]-p[2,0])+(p[2,0]-p[1,0])*(p[0,1]-p[2,1])
    if abs(denominator)<1e-9: continue
    yy,xx=np.mgrid[ymin:ymax+1,xmin:xmax+1]
    a=((p[1,1]-p[2,1])*(xx-p[2,0])+(p[2,0]-p[1,0])*(yy-p[2,1]))/denominator
    b=((p[2,1]-p[0,1])*(xx-p[2,0])+(p[0,0]-p[2,0])*(yy-p[2,1]))/denominator
    c=1-a-b; depth=a*p[0,2]+b*p[1,2]+c*p[2,2]
    tile=z[ymin:ymax+1,xmin:xmax+1]
    mask=(a>=-1e-6)&(b>=-1e-6)&(c>=-1e-6)&(depth>tile)
    tile[mask]=depth[mask]
    normals[ymin:ymax+1,xmin:xmax+1][mask]=normal
    material[ymin:ymax+1,xmin:xmax+1][mask]=owner
edge=np.zeros_like(z,dtype=bool)
# Surface discontinuities, rather than triangulation edges; back geometry is occluded.
for axis in (0,1):
    other=np.roll(z,1,axis); n=np.roll(normals,1,axis); m=np.roll(material,1,axis)
    both=np.isfinite(z)&np.isfinite(other)
    delta=np.zeros_like(z); np.subtract(z,other,out=delta,where=both)
    edge|=(np.isfinite(z)!=np.isfinite(other)) | (both & ((abs(delta)>.018) | (abs((normals*n).sum(axis=2))<.82) | (material!=m)))
rgba=np.zeros((height,width,4),dtype=np.uint8); rgba[:,:,:3]=[233,241,244]; rgba[:,:,3]=edge.astype(np.uint8)*220
args.output.mkdir(parents=True,exist_ok=True)
Image.fromarray(rgba).save(args.output/'board-top.png')
connectors={}
for name in ['J1','J2','J3','J4','J5','J6','J7']:
    ids=np.unique(f[np.array(groups)==name]); points=screen[ids]
    connectors[name]={'bounds':[float(points[:,0].min()/width),float(points[:,1].min()/height),float(points[:,0].max()/width),float(points[:,1].max()/height)]}
metadata={'source':'User esp-guage-models.zip / ESP Guage PCB_PCB.pdf.obj','sha256':hashlib.sha256(raw).hexdigest(),'view':'orthographic top, +Z looking down; USB left','size':[width,height],'connectors':connectors}
(args.output/'board-top.json').write_text(json.dumps(metadata,indent=2)+'\n')
print(f'Rendered {len(f)} triangles to {width}x{height}')
