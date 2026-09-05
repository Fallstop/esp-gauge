"""Static top-down CAD view with model-derived hit targets; no render loop."""
import json
from pathlib import Path
from PySide6.QtCore import Qt, QRectF, Signal
from PySide6.QtGui import QPainter, QPen, QColor, QPixmap
from PySide6.QtWidgets import QWidget

# Schematic filter rows, not connector numerical order. See docs/board-artwork.md.
CONNECTORS = ('J1', 'J3', 'J5', 'J2', 'J4', 'J6')

class BoardView(QWidget):
    selected = Signal(int)

    def __init__(self):
        super().__init__()
        assets=Path(__file__).parent/'assets'
        self.model=json.loads((assets/'board-top.json').read_text())
        self.pixmap=QPixmap(str(assets/'board-top.png'))
        self.index=0
        self.setMinimumSize(310,340)
        self.setMouseTracking(True)
        self.setToolTip('Top view · USB on the left. Click a gauge connector to select its output.')

    def image_rect(self):
        size=self.pixmap.size()
        scale=min((self.width()-20)/size.width(),(self.height()-50)/size.height())
        w,h=size.width()*scale,size.height()*scale
        return QRectF((self.width()-w)/2,(self.height()-h)/2,w,h)

    def connector_rect(self,name):
        image=self.image_rect()
        left,top,right,bottom=self.model['connectors'][name]['bounds']
        return QRectF(image.x()+left*image.width(),image.y()+top*image.height(),(right-left)*image.width(),(bottom-top)*image.height())

    def select(self,index):
        if self.index!=index:
            self.index=index
            self.update()

    def paintEvent(self,event):
        painter=QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        painter.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform)
        painter.drawPixmap(self.image_rect(),self.pixmap,QRectF(self.pixmap.rect()))
        for index,name in enumerate(CONNECTORS):
            rect=self.connector_rect(name)
            painter.setPen(QPen(QColor('#5cd8bc' if index==self.index else '#aebbc4'),1.3))
            painter.setBrush(QColor(92,216,188,30) if index==self.index else Qt.BrushStyle.NoBrush)
            painter.drawRoundedRect(rect.adjusted(-3,-3,3,3),3,3)
            top=rect.center().y()<self.height()/2
            label=QRectF(rect.x()-10,rect.y()-25 if top else rect.bottom()+5,rect.width()+20,20)
            painter.drawText(label,Qt.AlignmentFlag.AlignCenter,f'{index+1} / {name}')

    def mousePressEvent(self,event):
        for index,name in enumerate(CONNECTORS):
            if self.connector_rect(name).adjusted(-5,-8,5,8).contains(event.position()):
                self.select(index)
                self.selected.emit(index)
                return

    def mouseMoveEvent(self,event):
        hit=any(self.connector_rect(name).adjusted(-5,-8,5,8).contains(event.position()) for name in CONNECTORS)
        self.setCursor(Qt.CursorShape.PointingHandCursor if hit else Qt.CursorShape.ArrowCursor)
